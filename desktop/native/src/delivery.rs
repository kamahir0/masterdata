//! One native delivery job at a time. Snapshot capture/compile/publish never
//! occupies the workspace actor, and read caches never authorize destination writes.
use crate::actor::UiError;
use masterdata_engine::{
    Error, Result,
    delivery::{self, BuildOptions, BuildStage, SavedConfig},
    native::{
        artifact::ArtifactSet,
        dotnet,
        publish::{PublishFault, PublishPlan},
    },
    project::{Metadata, Project},
    source::content_identity,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU8, AtomicU64, Ordering},
    },
};

const ACTIVE: u8 = 1;
const MUTATING: u8 = 2;
const CAPTURING: u8 = 4;
#[derive(Default)]
pub struct Gate(AtomicU8);
impl Gate {
    pub(crate) fn reserve(
        &self,
        mutation: bool,
        capture: bool,
    ) -> std::result::Result<(), UiError> {
        let bits =
            ACTIVE | if mutation { MUTATING } else { 0 } | if capture { CAPTURING } else { 0 };
        self.0.compare_exchange(0,bits,Ordering::AcqRel,Ordering::Acquire)
            .map(|_|()).map_err(|_|UiError::new("E-DELIVERY-BUSY","Build / Publish / Migration operation is already running; retry explicitly after completion"))
    }
    pub(crate) fn release(&self) {
        self.0.store(0, Ordering::Release);
    }
    pub fn mutating(&self) -> bool {
        self.0.load(Ordering::Acquire) & MUTATING != 0
    }
    pub fn capturing(&self) -> bool {
        self.0.load(Ordering::Acquire) & CAPTURING != 0
    }
    pub fn active(&self) -> bool {
        self.0.load(Ordering::Acquire) & ACTIVE != 0
    }
}
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Request {
    Context,
    Build {
        profile: Option<String>,
        #[serde(default)]
        dry_run: bool,
    },
    PublishPreview,
    ConfirmPublish {
        token: String,
    },
    CancelPreview,
    RecheckArtifacts,
}
impl Request {
    pub(crate) fn flags(&self) -> (bool, bool) {
        (
            matches!(self, Self::Build { .. } | Self::ConfirmPublish { .. }),
            matches!(self, Self::Build { .. }),
        )
    }
    fn name(&self) -> &'static str {
        match self {
            Self::Context => "context",
            Self::Build { .. } => "build",
            Self::PublishPreview => "publishPreview",
            Self::ConfirmPublish { .. } => "publish",
            Self::CancelPreview => "cancelPreview",
            Self::RecheckArtifacts => "recheckArtifacts",
        }
    }
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Captured {
    pub project: Metadata,
    pub root: PathBuf,
    pub config_identity: String,
    pub input_identity: BTreeMap<String, String>,
}
#[derive(Default)]
struct State {
    epoch: u64,
    id: u64,
    kind: String,
    phase: String,
    status: String,
    root: PathBuf,
    profile: Option<String>,
    captured: Option<Captured>,
    context: Option<Value>,
    result: Option<Value>,
    error: Option<Error>,
    last_build: Option<BuildAttempt>,
    preview: Option<(String, Arc<PublishPlan>)>,
}
#[derive(Clone)]
struct BuildAttempt {
    id: u64,
    profile: Option<String>,
    captured: Option<Captured>,
    status: String,
    result: Option<Value>,
    error: Option<Error>,
}
impl BuildAttempt {
    fn report(&self) -> Value {
        json!({"id":self.id,"profile":self.profile,"captured":self.captured,
            "status":self.status,"result":self.result,
            "error":self.error.as_ref().map(|error|report_error(error,0,200))})
    }
}
#[derive(Clone)]
pub struct Session {
    state: Arc<Mutex<State>>,
    next: Arc<AtomicU64>,
    pub gate: Arc<Gate>,
}
impl Default for Session {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            next: Arc::new(AtomicU64::new(0)),
            gate: Arc::new(Gate::default()),
        }
    }
}
fn report_error(error: &Error, start: usize, count: usize) -> Value {
    let diagnostics = error.diagnostics.as_deref().unwrap_or(&[]);
    json!({"code":error.code,"message":error.message,
        "diagnostics":diagnostics.iter().skip(start).take(count.min(200)).collect::<Vec<_>>(),
        "diagnosticTotal":diagnostics.len()})
}
impl Session {
    pub fn snapshot(&self, epoch: u64) -> std::result::Result<Value, UiError> {
        let state = self.state.lock().map_err(|_| {
            UiError::new(
                "E-DELIVERY-UNKNOWN",
                "delivery state is unavailable; inspect actual artifacts",
            )
        })?;
        if state.epoch != epoch || state.id == 0 {
            return Ok(
                json!({"epoch":epoch,"id":0,"status":"idle","running":self.gate.active(),"mutating":self.gate.mutating(),"capturing":self.gate.capturing(),"kind":null,"phase":null,"profile":null,"root":null,"captured":null,"context":null,"result":null,"error":null,"lastBuild":null,"preview":null}),
            );
        }
        Ok(
            json!({"epoch":epoch,"id":state.id,"status":state.status,"running":self.gate.active(),"mutating":self.gate.mutating(),"capturing":self.gate.capturing(),
            "kind":state.kind,"phase":state.phase,"profile":state.profile,"root":state.root,
            "captured":state.captured,"context":state.context,"result":state.result,
            "error":state.error.as_ref().map(|error|report_error(error,0,200)),
            "lastBuild":state.last_build.as_ref().map(BuildAttempt::report),
            "preview":state.preview.as_ref().map(|(token,plan)|json!({"token":token,"detail":plan.preview()}))}),
        )
    }
    pub(crate) fn start(
        &self,
        epoch: u64,
        project: &Project,
        request: Request,
    ) -> std::result::Result<Value, UiError> {
        let id = self.next.fetch_add(1, Ordering::AcqRel) + 1;
        let root = project.root.clone();
        let project_id = project.config.project.id.clone();
        {
            let mut state = self
                .state
                .lock()
                .map_err(|_| UiError::new("E-DELIVERY-UNKNOWN", "delivery state is unavailable"))?;
            if state.epoch != epoch {
                *state = State::default();
                state.epoch = epoch;
            }
            state.id = id;
            state.kind = request.name().into();
            state.root = root.clone();
            state.profile = match &request {
                Request::Build { profile, .. } => profile.clone(),
                _ => None,
            };
            state.phase = if self.gate.capturing() {
                "capturing"
            } else {
                "preparing"
            }
            .into();
            state.status = "running".into();
            state.result = None;
            state.error = None;
            state.captured = None;
            if !matches!(
                request,
                Request::ConfirmPublish { .. } | Request::CancelPreview
            ) {
                state.preview = None;
            }
        }
        // The actor owns the reservation until successful spawn transfers it
        // to the worker. Nothing fallible may run after that transfer: a second
        // release could clear the next job's reservation or an active write.
        let initial = self.snapshot(epoch)?;
        let worker = self.clone();
        let started=std::thread::Builder::new().name("masterdata-delivery".into()).spawn(move || {
            let outcome=std::panic::catch_unwind(std::panic::AssertUnwindSafe(||worker.run(epoch,id,&root,&project_id,request)))
                .unwrap_or_else(|_|Err(Error::new("E-DELIVERY-UNKNOWN","delivery worker stopped unexpectedly; inspect actual artifacts / destinations before another write")));
            if let Ok(mut state)=worker.state.lock()
                && state.epoch==epoch && state.id==id {
                match outcome {
                    Ok(value)=>{
                        let outcome=value.get("outcome").and_then(Value::as_str);
                        state.status=if matches!(outcome,Some("Failure"|"Conflict"|"OutcomeUnknown"|"RecoveryRequired")) {"failed"} else {"succeeded"}.into();
                        state.result=Some(value);state.phase="complete".into();
                    }
                    Err(error)=>{state.error=Some(error);state.status="failed".into();state.phase="complete".into();}
                }
                if state.kind=="build" {
                    state.last_build=Some(BuildAttempt{id,profile:state.profile.clone(),captured:state.captured.clone(),status:state.status.clone(),result:state.result.clone(),error:state.error.clone()});
                }
            }
            worker.gate.release();
        });
        if let Err(error) = started {
            if let Ok(mut state) = self.state.lock() {
                let error = Error::new("E-DELIVERY-START", error.to_string());
                state.status = "failed".into();
                state.phase = "complete".into();
                state.error = Some(error.clone());
                if state.kind == "build" {
                    state.last_build = Some(BuildAttempt {
                        id,
                        profile: state.profile.clone(),
                        captured: None,
                        status: state.status.clone(),
                        result: None,
                        error: Some(error),
                    });
                }
            }
            return Err(UiError::new("E-DELIVERY-START", &error.to_string()));
        }
        Ok(initial)
    }
    fn run(
        &self,
        epoch: u64,
        id: u64,
        root: &std::path::Path,
        project_id: &str,
        request: Request,
    ) -> Result<Value> {
        match request {
            Request::Context => {
                let saved = SavedConfig::load(root)?;
                if saved.config.project.id != project_id {
                    return Err(Error::new(
                        "E-PROJECT-BINDING",
                        "saved Project identity changed; reload Project",
                    ));
                }
                let capability = match dotnet::environment() {
                    Ok(_) => json!({"available":true,"reason":null}),
                    Err(error) => json!({"available":false,"reason":error.to_string()}),
                };
                let receipt = match ArtifactSet::load(&saved) {
                    Ok(set) => {
                        json!({"eligible":true,"root":set.root,"identity":content_identity(&serde_json::to_vec(&set.receipt).unwrap()),"reason":null})
                    }
                    Err(error) => {
                        json!({"eligible":false,"root":saved.root.join(&saved.config.build.artifact_dir),"identity":null,"reason":error.to_string()})
                    }
                };
                let context = json!({"project":saved.config.project,"configIdentity":saved.identity(),"profiles":saved.config.build.profiles.keys().collect::<Vec<_>>(),
                    "targets":saved.config.publish.targets,"buildCapability":capability,"receipt":receipt});
                self.state
                    .lock()
                    .map_err(|_| Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable"))?
                    .context = Some(context.clone());
                Ok(json!({"outcome":"Success","context":context}))
            }
            Request::Build { profile, dry_run } => {
                let result = delivery::build_observed(
                    root,
                    &BuildOptions {
                        profile,
                        dry_run,
                        ..Default::default()
                    },
                    |stage| {
                        match stage {
                            BuildStage::Snapshot(project) => {
                                if project.config.project.id != project_id {
                                    return Err(Error::new(
                                        "E-PROJECT-BINDING",
                                        "saved Project identity changed; reload Project",
                                    ));
                                }
                                let mut state = self.state.lock().map_err(|_| {
                                    Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable")
                                })?;
                                if state.epoch != epoch || state.id != id {
                                    return Err(Error::new(
                                        "E-PROJECT-OBSOLETE",
                                        "Build belongs to an obsolete Project",
                                    ));
                                }
                                state.captured = Some(Captured {
                                    project: project.config.project.clone(),
                                    root: project.root.clone(),
                                    config_identity: project.config_identity.clone(),
                                    input_identity: project
                                        .sources
                                        .iter()
                                        .map(|(path, source)| {
                                            (path.clone(), source.identity.clone())
                                        })
                                        .collect(),
                                });
                            }
                            BuildStage::Ready => {
                                self.state
                                    .lock()
                                    .map_err(|_| {
                                        Error::new(
                                            "E-DELIVERY-UNKNOWN",
                                            "delivery state unavailable",
                                        )
                                    })?
                                    .phase = "building".into();
                                // The immutable validated Plan no longer reads source.
                                // Save is permitted; config/structural/delivery stay busy.
                                self.gate.0.fetch_and(!CAPTURING, Ordering::AcqRel);
                            }
                        }
                        Ok(())
                    },
                )?;
                // Artifact eligibility is independent from the current draft
                // and from whether this Build succeeded. Refresh it from the
                // actual receipt so a previous successful set remains usable.
                let receipt = SavedConfig::load(root).and_then(|saved| ArtifactSet::load(&saved));
                if let Ok(mut state) = self.state.lock()
                    && let Some(context) = state.context.as_mut()
                {
                    context["receipt"] = match receipt {
                        Ok(set) => {
                            json!({"eligible":true,"root":set.root,"identity":content_identity(&serde_json::to_vec(&set.receipt).unwrap()),"reason":null})
                        }
                        Err(error) => {
                            json!({"eligible":false,"root":result.artifact_root,"identity":null,"reason":error.to_string()})
                        }
                    };
                }
                Ok(serde_json::to_value(result).unwrap())
            }
            Request::PublishPreview => {
                let plan = match PublishPlan::prepare(root) {
                    Ok(plan) => Arc::new(plan),
                    Err(error) => {
                        let targets = SavedConfig::load(root)
                            .ok()
                            .map(|saved| saved.config.publish.targets);
                        self.state
                            .lock()
                            .map_err(|_| {
                                Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable")
                            })?
                            .result = Some(
                            json!({"outcome":"Failure","preflight":"unavailable","targetsNotAttempted":targets,"sourceFreshness":"not_checked","unityVerification":"not_observed"}),
                        );
                        return Err(error);
                    }
                };
                if plan.preview().project_id != project_id {
                    return Err(Error::new(
                        "E-PROJECT-BINDING",
                        "saved Project identity changed; reload Project",
                    ));
                }
                let token = format!("publish-{epoch}-{id}");
                let preview = json!({"token":token,"detail":plan.preview()});
                self.state
                    .lock()
                    .map_err(|_| Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable"))?
                    .preview = Some((token, plan));
                Ok(json!({"outcome":"NotAttempted","preview":preview}))
            }
            Request::ConfirmPublish { token } => {
                let plan = {
                    let state = self.state.lock().map_err(|_| {
                        Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable")
                    })?;
                    state
                        .preview
                        .as_ref()
                        .filter(|(expected, _)| expected == &token)
                        .map(|(_, plan)| plan.clone())
                        .ok_or_else(|| {
                            Error::new(
                                "E-PUBLISH-STALE",
                                "preview is unavailable; request and confirm a new preview",
                            )
                        })?
                };
                let report = plan.execute(PublishFault::None);
                self.state
                    .lock()
                    .map_err(|_| Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable"))?
                    .preview = None;
                Ok(serde_json::to_value(report?).unwrap())
            }
            Request::CancelPreview => {
                self.state
                    .lock()
                    .map_err(|_| Error::new("E-DELIVERY-UNKNOWN", "delivery state unavailable"))?
                    .preview = None;
                Ok(
                    json!({"outcome":"NotAttempted","message":"Publish cancelled; canonical Build retained"}),
                )
            }
            Request::RecheckArtifacts => {
                let saved = SavedConfig::load(root)?;
                let artifact = ArtifactSet::load(&saved)?;
                Ok(
                    json!({"outcome":"Success","message":"actual complete artifact set and every receipt hash verified",
                    "root":artifact.root,"receipt":artifact.receipt,"sourceFreshness":"not_checked","unityVerification":"not_observed"}),
                )
            }
        }
    }
    pub fn problems(
        &self,
        epoch: u64,
        id: u64,
        start: usize,
        count: usize,
    ) -> std::result::Result<Value, UiError> {
        let state = self
            .state
            .lock()
            .map_err(|_| UiError::new("E-DELIVERY-UNKNOWN", "delivery state unavailable"))?;
        let attempt = state.last_build.as_ref().filter(|attempt| attempt.id == id);
        if state.epoch != epoch || attempt.is_none() {
            return Err(UiError::new("E-PROJECT-OBSOLETE", "Build result changed"));
        }
        Ok(attempt
            .unwrap()
            .error
            .as_ref()
            .map(|error| report_error(error, start, count))
            .unwrap_or_else(|| json!({"diagnostics":[],"diagnosticTotal":0})))
    }
    pub fn problem(
        &self,
        epoch: u64,
        id: u64,
        index: usize,
    ) -> std::result::Result<(masterdata_engine::project::Diagnostic, Captured), UiError> {
        let state = self
            .state
            .lock()
            .map_err(|_| UiError::new("E-DELIVERY-UNKNOWN", "delivery state unavailable"))?;
        let attempt = state
            .last_build
            .as_ref()
            .filter(|attempt| state.epoch == epoch && attempt.id == id)
            .ok_or_else(|| UiError::new("E-PROJECT-OBSOLETE", "Build result changed"))?;
        let diagnostic = attempt
            .error
            .as_ref()
            .and_then(|error| error.diagnostics.as_ref())
            .and_then(|diagnostics| diagnostics.get(index))
            .ok_or_else(|| UiError::new("E-BUILD-PROBLEM", "Build diagnostic unavailable"))?;
        let captured = attempt
            .captured
            .as_ref()
            .ok_or_else(|| UiError::new("E-BUILD-PROBLEM", "Build snapshot unavailable"))?;
        Ok((diagnostic.clone(), captured.clone()))
    }
}
