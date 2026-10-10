//! Native session scheduling. Queue locks only protect pending work, never parsing or I/O.
use masterdata_engine::{
    Error, instrument,
    project::{Diagnostic, Project},
    source::Value as SourceValue,
    workspace::Workspace,
};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeSet, VecDeque},
    path::{Path, PathBuf},
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread,
    time::Instant,
};
use tokio::sync::oneshot;

#[derive(Clone, Debug, Serialize)]
pub struct UiError {
    pub code: String,
    pub message: String,
}
impl From<Error> for UiError {
    fn from(e: Error) -> Self {
        Self {
            code: e.code.into(),
            message: e.message,
        }
    }
}
impl UiError {
    pub(crate) fn new(code: &str, message: &str) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}
type Reply = Result<String, UiError>;
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Intent {
    DeliveryStart {
        request: crate::delivery::Request,
    },
    DeliveryState,
    DeliveryProblems {
        id: u64,
        start: usize,
        count: usize,
    },
    DeliveryProblemTarget {
        id: u64,
        index: usize,
    },
    Open {
        path: String,
        discard: bool,
    },
    CreateProject {
        path: String,
        metadata: masterdata_engine::project::Metadata,
        discard: bool,
    },
    Inventory,
    ConfigView {
        profile: Option<String>,
        #[serde(default)]
        starts: [usize; 4],
    },
    ConfigEdit {
        revision: u64,
        operation: masterdata_engine::config::Operation,
    },
    ConfigSave {
        revision: u64,
    },
    ConfigCompare {
        external: bool,
    },
    ConfigReload {
        revision: u64,
        discard_authorized: bool,
    },
    ConfigRecheck,
    CreationChoices,
    CreationDefaults {
        category: String,
        path: String,
        table: Option<String>,
    },
    CreationPreview {
        request: masterdata_engine::creation::Request,
    },
    CreationField {
        fields: Vec<masterdata_engine::creation::Declaration>,
    },
    Create {
        request: masterdata_engine::creation::Request,
    },
    RecheckCreation {
        path: String,
    },
    PathMoveChoices {
        source: String,
    },
    PathMovePreview {
        source: String,
        destination: String,
    },
    MoveSource {
        token: String,
    },
    RecheckMove {
        token: String,
    },
    SaveSource {
        source: String,
    },
    DiscardSource {
        source: String,
    },
    MigrationPlan {
        command: masterdata_engine::migration::Command,
    },
    TableDeclarationDetail {
        source: String,
        table: String,
    },
    TableDeclarationPlan {
        command: masterdata_engine::table_declaration::Command,
    },
    TypeMigrationPlan {
        source: String,
        identity: String,
        command: masterdata_engine::type_migration::Command,
        #[serde(default)]
        input: Option<masterdata_engine::initializer::Input>,
    },
    TypeInitializer {
        source: String,
        identity: String,
        declaration: masterdata_engine::creation::Declaration,
    },
    FieldScope {
        source: String,
        revision: u64,
        generation: u64,
        operation: masterdata_engine::workspace::FieldOperation,
    },
    FieldOperation {
        source: String,
        revision: u64,
        generation: u64,
        request: masterdata_engine::workspace::FieldIntent,
    },
    MigrationCompare {
        token: String,
        source: String,
    },
    MigrationApply {
        token: String,
        authorize_destructive: bool,
    },
    MigrationResult {
        token: String,
    },
    MigrationRecovery {
        id: String,
        restore_old: bool,
        authorized: bool,
    },
    AuthoringState {
        source: String,
        revision: u64,
        generation: u64,
    },
    Select {
        path: String,
        start: usize,
        count: usize,
        token: u64,
    },
    EditText {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        field: String,
        text: String,
    },
    EditValue {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        path: Vec<String>,
        value: SourceValue,
    },
    Paste {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        field: String,
        text: String,
    },
    Copy {
        source: String,
        revision: u64,
        generation: u64,
        anchor: String,
        focus: String,
        first: String,
        last: String,
    },
    Search {
        source: String,
        text: String,
    },
    Locate {
        source: String,
        row: String,
    },
    ProblemTarget {
        source: String,
        generation: u64,
        occurrence: Option<usize>,
        path: Vec<String>,
    },
    AddRow {
        source: String,
        revision: u64,
        generation: u64,
        before: Option<String>,
    },
    DeleteRow {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        restore: bool,
    },
    MoveRow {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        before: Option<String>,
    },
    NudgeRow {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        delta: isize,
    },
    NextRow {
        source: String,
        row: String,
    },
    Columns {
        source: String,
        revision: u64,
        generation: u64,
        order: Vec<String>,
    },
    Complex {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        path: Vec<String>,
        start: usize,
        count: usize,
    },
    ComplexEdit {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        path: Vec<String>,
        operation: masterdata_engine::workspace::complex::Operation,
    },
    Tags {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        start: usize,
    },
    TagEdit {
        source: String,
        revision: u64,
        generation: u64,
        row: String,
        operation: masterdata_engine::workspace::tags::TagOperation,
    },
    Schema {
        source: String,
        revision: u64,
        generation: u64,
        field: String,
        nullable: bool,
        array: bool,
        type_name: Option<String>,
    },
    Undo {
        source: String,
        redo: bool,
    },
    Save {
        table: String,
        source: Option<String>,
    },
    SaveAll,
    Compare {
        source: String,
    },
    Overwrite {
        source: String,
        identity: String,
    },
    ReloadSource {
        source: String,
    },
    Problems {
        start: usize,
        count: usize,
    },
    Validate,
    Remember {
        source: String,
        search: String,
        row: Option<String>,
        field: Option<String>,
        top: f64,
        left: f64,
    },
}
impl Intent {
    fn operation_flags(&self) -> Option<(bool, bool)> {
        match self {
            Self::DeliveryStart { request } => Some(request.flags()),
            Self::MigrationApply { .. }
            | Self::MigrationRecovery { .. }
            | Self::FieldOperation { .. }
            | Self::CreateProject { .. } => Some((true, true)),
            _ => None,
        }
    }
    fn source_write(&self) -> bool {
        matches!(
            self,
            Self::Save { .. }
                | Self::ConfigSave { .. }
                | Self::SaveAll
                | Self::SaveSource { .. }
                | Self::Overwrite { .. }
                | Self::Create { .. }
                | Self::CreateProject { .. }
                | Self::MoveSource { .. }
        )
    }
    fn token(&self) -> Option<u64> {
        if let Self::Select { token, .. } = self {
            Some(*token)
        } else {
            None
        }
    }
}
struct Job {
    intent: Intent,
    reply: oneshot::Sender<Reply>,
    enqueued: Instant,
    epoch: Option<u64>,
}
struct DiagnosticResult {
    epoch: u64,
    generation: u64,
    problems: Vec<Diagnostic>,
}
#[derive(Default)]
struct Pending {
    commands: VecDeque<Job>,
    selection: Option<Job>,
    diagnostics: Option<DiagnosticResult>,
    external: Option<External>,
    stopped: bool,
}
struct External {
    epoch: u64,
    paths: BTreeSet<PathBuf>,
    error: Option<String>,
}
struct Queue {
    pending: Mutex<Pending>,
    wake: Condvar,
    latest: AtomicU64,
}
#[derive(Default)]
struct ValidationPending {
    snapshot: Option<(u64, Project)>,
    stopped: bool,
}
struct Validator {
    pending: Mutex<ValidationPending>,
    wake: Condvar,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub open: bool,
    pub epoch: u64,
    pub generation: u64,
    pub dirty: Vec<String>,
    pub config_dirty: bool,
    pub config_uncertain: bool,
    pub config_identity: String,
    pub recovery_required: bool,
    pub diagnostics_pending: bool,
    pub problem_count: usize,
    pub uncertain: Vec<String>,
    pub external_version: u64,
    pub environment_error: Option<String>,
}
#[derive(Clone)]
pub struct Session {
    queue: Arc<Queue>,
    pub protected: Arc<AtomicBool>,
    delivery: crate::delivery::Session,
    epoch: Arc<AtomicU64>,
}
impl Default for Session {
    fn default() -> Self {
        Self::new(Arc::new(|_| {}))
    }
}
impl Session {
    pub fn new(publish: Arc<dyn Fn(Status) + Send + Sync>) -> Self {
        let queue = Arc::new(Queue {
            pending: Mutex::new(Pending::default()),
            wake: Condvar::new(),
            latest: AtomicU64::new(0),
        });
        let validator = Arc::new(Validator {
            pending: Mutex::new(ValidationPending::default()),
            wake: Condvar::new(),
        });
        let protected = Arc::new(AtomicBool::new(false));
        let current_epoch = Arc::new(AtomicU64::new(0));
        let published_epoch = current_epoch.clone();
        let v = validator.clone();
        let q = queue.clone();
        thread::Builder::new()
            .name("masterdata-diagnostics".into())
            .spawn(move || {
                loop {
                    let task = {
                        let mut p = v.pending.lock().unwrap();
                        while p.snapshot.is_none() && !p.stopped {
                            p = v.wake.wait(p).unwrap();
                        }
                        if p.stopped {
                            break;
                        }
                        p.snapshot.take().unwrap()
                    };
                    let (epoch, mut project) = task;
                    project.rebuild_declarations();
                    let generation = project.generation;
                    let problems = match project.validate(None) {
                        Ok((p, _)) => p,
                        Err(e) => vec![Diagnostic::error("masterdata.toml", &e, generation)],
                    };
                    let mut p = q.pending.lock().unwrap();
                    p.diagnostics = Some(DiagnosticResult {
                        epoch,
                        generation,
                        problems,
                    });
                    q.wake.notify_one();
                }
            })
            .expect("diagnostics worker");
        let delivery = crate::delivery::Session::default();
        let jobs = delivery.clone();
        let q = queue.clone();
        let guard = protected.clone();
        thread::Builder::new().name("masterdata-workspace".into()).spawn(move || {
            let mut workspace: Option<Workspace> = None;
            let mut epoch = 0;
            let mut scheduled = 0;
            let mut _watcher = None;
            loop {
                enum Work { Job(Job), Diagnostics(DiagnosticResult), External(External) }
                let work = {
                    let mut p = q.pending.lock().unwrap();
                    while p.commands.is_empty() && p.selection.is_none() && p.diagnostics.is_none() && p.external.is_none() && !p.stopped {
                        p = q.wake.wait(p).unwrap();
                    }
                    if p.stopped { break; }
                    if let Some(result) = p.diagnostics.take() { Work::Diagnostics(result) }
                    else if let Some(job) = p.commands.pop_front().or_else(|| p.selection.take()) { Work::Job(job) }
                    else { Work::External(p.external.take().unwrap()) }
                };
                match work {
                    Work::Diagnostics(result) => {
                        if epoch == result.epoch && let Some(w) = workspace.as_mut() {
                            w.accept_diagnostics(result.generation, result.problems);
                        }
                    }
                    Work::External(result) => {
                        if epoch == result.epoch && let Some(w) = workspace.as_mut() {
                            if let Some(error) = result.error {
                                w.invalidate_environment(Error::new("E-WATCH-UNAVAILABLE", error));
                            } else { w.refresh_paths(&result.paths.into_iter().collect::<Vec<_>>()); }
                        }
                    }
                    Work::Job(job) => {
                        if job.epoch.is_some_and(|expected| expected != epoch) {
                            if job.intent.operation_flags().is_some() {jobs.gate.release();}
                            let _ = job.reply.send(Err(UiError::new("E-PROJECT-OBSOLETE", "request belongs to a previous Project session")));
                            continue;
                        }
                        let token = job.intent.token();
                        if token.is_some_and(|t| t != q.latest.load(Ordering::Acquire)) {
                            let _ = job.reply.send(Err(UiError::new("E-SELECTION-OBSOLETE", "selection superseded")));
                            continue;
                        }
                        let queued_ms = job.enqueued.elapsed().as_secs_f64() * 1000.0;
                        let start = Instant::now();
                        let previous_epoch = epoch;
                        let operation=job.intent.operation_flags().is_some();
                        let background=matches!(job.intent,Intent::DeliveryStart{..});
                        if matches!(job.intent, Intent::Validate) { scheduled = 0; }
                        let (result, measurement) = instrument::measure(|| execute(&mut workspace, &mut epoch, &jobs, job.intent));
                        published_epoch.store(epoch,Ordering::Release);
                        if operation && (!background || result.is_err()) {jobs.gate.release();}
                        if epoch != previous_epoch {
                            scheduled = 0;
                            _watcher = None;
                            if let Some(w) = workspace.as_mut() {
                                match watch(q.clone(), &w.read, epoch) {
                                    Ok(watcher) => _watcher = Some(watcher),
                                    Err(error) => w.invalidate_environment(error),
                                }
                            }
                        }
                        guard.store(jobs.gate.mutating() || workspace.as_ref().is_some_and(Workspace::protected), Ordering::Release);
                        let reply = if token.is_some_and(|t| t != q.latest.load(Ordering::Acquire)) {
                            Err(UiError::new("E-SELECTION-OBSOLETE", "selection superseded"))
                        } else {
                            result.and_then(|data| {
                                let encode = Instant::now();
                                let encoded = serde_json::to_string(&data).map_err(|e| UiError::new("E-IPC", &e.to_string()))?;
                                let serialization_ms = encode.elapsed().as_secs_f64() * 1000.0;
                                let meta = json!({"epoch":epoch,"queuedMs":queued_ms,"backendMs":measurement.elapsed_ms,"serializationMs":serialization_ms,"bytes":encoded.len(),"token":token,"work":measurement.work,"stagesMs":measurement.stages_ms,"nativeCompleteMs":start.elapsed().as_secs_f64()*1000.0});
                                Ok(format!("{{\"data\":{encoded},\"host\":{meta}}}"))
                            })
                        };
                        let _ = job.reply.send(reply);
                    }
                }
                if let Some(w) = workspace.as_ref() {
                    guard.store(jobs.gate.mutating() || w.protected(), Ordering::Release);
                    publish(Status { open:true, epoch, generation:w.generation, dirty:w.dirty_paths(), config_dirty:w.config_dirty(),config_uncertain:w.config_uncertain(),config_identity:w.configuration.base.content.clone(), recovery_required:w.recovery_required, diagnostics_pending:w.diagnostics_pending, problem_count:w.diagnostics.len(), uncertain:w.uncertain_paths(), external_version:w.external_version, environment_error:w.environment_error.as_ref().map(ToString::to_string) });
                    if w.diagnostics_pending && scheduled != w.generation {
                        let mut p = validator.pending.lock().unwrap();
                        p.snapshot = Some((epoch, w.diagnostic_input()));
                        scheduled = w.generation;
                        validator.wake.notify_one();
                    }
                }
            }
            let mut p = validator.pending.lock().unwrap();
            p.stopped = true;
            validator.wake.notify_one();
        }).expect("workspace worker");
        Self {
            queue,
            protected,
            delivery,
            epoch: current_epoch,
        }
    }
    pub fn submit(&self, intent: Intent) -> oneshot::Receiver<Reply> {
        self.submit_at(intent, None)
    }
    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }
    fn submit_at(&self, intent: Intent, epoch: Option<u64>) -> oneshot::Receiver<Reply> {
        let (tx, rx) = oneshot::channel();
        if matches!(intent, Intent::Open { .. } | Intent::CreateProject { .. })
            && self.delivery.gate.mutating()
        {
            let _ = tx.send(Err(UiError::new(
                "E-DELIVERY-BUSY",
                "wait for the running mutation to finish, then switch Project explicitly",
            )));
            return rx;
        }
        if intent.source_write() && self.delivery.gate.capturing() {
            let _ = tx.send(Err(UiError::new(
                "E-DELIVERY-CAPTURING",
                "saved input capture / structural operation is running; Save was not queued",
            )));
            return rx;
        }
        let reserved = if let Some((mutation, capture)) = intent.operation_flags() {
            if let Err(error) = self.delivery.gate.reserve(mutation, capture) {
                let _ = tx.send(Err(error));
                return rx;
            }
            if mutation {
                self.protected.store(true, Ordering::Release);
            }
            true
        } else {
            false
        };
        let mut pending = self.queue.pending.lock().unwrap();
        if pending.stopped {
            if reserved {
                self.delivery.gate.release();
            }
            let _ = tx.send(Err(UiError::new(
                "E-WORKSPACE-CLOSED",
                "workspace worker closed",
            )));
            return rx;
        }
        if let Some(token) = intent.token() {
            let latest = self.queue.latest.fetch_max(token, Ordering::AcqRel);
            if token < latest {
                let _ = tx.send(Err(UiError::new(
                    "E-SELECTION-OBSOLETE",
                    "selection superseded",
                )));
                return rx;
            }
            let job = Job {
                intent,
                reply: tx,
                epoch,
                enqueued: Instant::now(),
            };
            if let Some(old) = pending.selection.replace(job) {
                let _ = old.reply.send(Err(UiError::new(
                    "E-SELECTION-OBSOLETE",
                    "selection superseded",
                )));
            }
        } else if pending.commands.len() >= 64 {
            if reserved {
                self.delivery.gate.release();
            }
            let _ = tx.send(Err(UiError::new(
                "E-WORKSPACE-BUSY",
                "pending authoring operations are bounded; try after current operation",
            )));
            return rx;
        } else {
            pending.commands.push_back(Job {
                intent,
                reply: tx,
                epoch,
                enqueued: Instant::now(),
            });
        }
        self.queue.wake.notify_one();
        rx
    }
    pub async fn request_at(&self, intent: Intent, epoch: u64) -> Reply {
        self.submit_at(intent, Some(epoch))
            .await
            .map_err(|_| UiError::new("E-WORKSPACE-CLOSED", "workspace worker closed"))?
    }
    pub async fn request(&self, intent: Intent) -> Reply {
        self.submit(intent)
            .await
            .map_err(|_| UiError::new("E-WORKSPACE-CLOSED", "workspace worker closed"))?
    }
    pub fn request_blocking(&self, intent: Intent) -> Reply {
        self.submit(intent)
            .blocking_recv()
            .map_err(|_| UiError::new("E-WORKSPACE-CLOSED", "workspace worker closed"))?
    }
    pub fn stop(&self) {
        let mut p = self.queue.pending.lock().unwrap();
        p.stopped = true;
        self.queue.wake.notify_one();
    }
    pub fn mutating(&self) -> bool {
        self.delivery.gate.mutating()
    }
}
fn convert<T: Serialize>(v: T) -> Result<Value, UiError> {
    serde_json::to_value(v).map_err(|e| UiError::new("E-IPC", &e.to_string()))
}
fn watch(
    queue: Arc<Queue>,
    project: &Project,
    epoch: u64,
) -> Result<notify::RecommendedWatcher, Error> {
    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        if event.as_ref().is_ok_and(|event| event.kind.is_access()) { return; }
        let mut p = queue.pending.lock().unwrap();
        if p.stopped || p.external.as_ref().is_some_and(|pending| pending.epoch > epoch) { return; }
        if p.external.as_ref().is_none_or(|pending| pending.epoch != epoch) {
            p.external = Some(External { epoch, paths: BTreeSet::new(), error: None });
        }
        let pending = p.external.as_mut().unwrap();
        match event {
            Ok(event) if !event.need_rescan() => {
                for path in event.paths {
                    if pending.paths.len() >= 256 && !pending.paths.contains(&path) {
                        pending.error = Some("filesystem event backlog exceeded its bound; explicit Project Reload required".into());
                        break;
                    }
                    pending.paths.insert(path);
                }
            }
            Ok(_) => pending.error = Some("filesystem watcher lost events; explicit Project Reload required".into()),
            Err(error) => pending.error = Some(error.to_string()),
        }
        queue.wake.notify_one();
    }).map_err(|e| Error::new("E-WATCH-UNAVAILABLE", e.to_string()))?;
    for root in &project.roots {
        watcher
            .watch(root, notify::RecursiveMode::Recursive)
            .map_err(|e| Error::new("E-WATCH-UNAVAILABLE", e.to_string()))?;
    }
    watcher
        .watch(&project.root, notify::RecursiveMode::NonRecursive)
        .map_err(|e| Error::new("E-WATCH-UNAVAILABLE", e.to_string()))?;
    Ok(watcher)
}
fn inventory(w: &Workspace) -> Value {
    json!({"project":w.read.config.project,"root":w.read.root,"roots":w.read.config.sources.roots,
        // An empty configured root outside the Project can be accepted by the
        // engine. Keep its configured identity rather than panicking while
        // deriving display paths; source provenance remains engine-owned.
        "sourceRootPaths":w.read.roots.iter().zip(&w.read.config.sources.roots).map(|(root,configured)|root.strip_prefix(&w.read.root).map(|p|p.to_string_lossy().replace('\\',"/")).unwrap_or_else(|_|configured.clone())).collect::<Vec<_>>(),"folders":w.read.folders,
        "sources":w.read.sources.values().map(|s|json!({"path":s.path,"kind":s.kind,"binding":s.binding,"error":s.error})).collect::<Vec<_>>(),
        "logicalTables":w.read.tables.values().map(|table|json!({"name":table.name,"source":table.source})).collect::<Vec<_>>(),
        "logicalTypes":w.read.type_sources.iter().map(|(name,source)|json!({"name":name,"source":source})).collect::<Vec<_>>(),
        "types":w.read.types.keys().collect::<Vec<_>>(),"dirty":w.dirty_paths(),"configDirty":w.config_dirty(),"configUncertain":w.config_uncertain(),"configIdentity":w.configuration.base.content,"uncertain":w.uncertain_paths(),"recoveryRequired":w.recovery_required,"recovery":w.recovery_information,"generation":w.generation,"externalVersion":w.external_version,"environmentError":w.environment_error.as_ref().map(ToString::to_string)})
}
fn execute(
    workspace: &mut Option<Workspace>,
    epoch: &mut u64,
    delivery: &crate::delivery::Session,
    intent: Intent,
) -> Result<Value, UiError> {
    if let Intent::Open { path, discard } = intent {
        if delivery.gate.mutating() {
            return Err(UiError::new(
                "E-DELIVERY-BUSY",
                "running mutation must finish before Project switch",
            ));
        }
        if workspace.as_ref().is_some_and(Workspace::protected) && !discard {
            return Err(UiError::new(
                "E-PROJECT-DIRTY",
                "Save All / Don't Save / Cancel required",
            ));
        }
        let next = Workspace::open(Path::new(&path))?;
        *workspace = Some(next);
        *epoch += 1;
        return Ok(inventory(workspace.as_ref().unwrap()));
    }
    if let Intent::CreateProject {
        path,
        metadata,
        discard,
    } = intent
    {
        if workspace.as_ref().is_some_and(Workspace::protected) && !discard {
            return Err(UiError::new(
                "E-PROJECT-DIRTY",
                "Save All / Don't Save / Cancel required",
            ));
        }
        let mut creation = masterdata_engine::native::create_project(Path::new(&path), metadata)?;
        if creation.outcome == masterdata_engine::native::Outcome::Success {
            // Opening the new workspace is part of creation success. Never drop
            // the old drafts when final resolution fails or only a repair view
            // can be opened. The already-created scaffold remains inspectable.
            match Workspace::open(&creation.root) {
                Ok(next)
                    if next.environment_error.is_none() && creation.check_current().is_ok() =>
                {
                    *workspace = Some(next);
                    *epoch += 1;
                    return Ok(
                        json!({"creation":creation,"inventory":inventory(workspace.as_ref().unwrap())}),
                    );
                }
                Ok(next) => {
                    creation.outcome = masterdata_engine::native::Outcome::Failure;
                    creation.message = next
                        .environment_error
                        .map(|error| error.to_string())
                        .unwrap_or_else(|| {
                            "created Project binding changed before workspace adoption".into()
                        });
                }
                Err(error) => {
                    creation.outcome = masterdata_engine::native::Outcome::Failure;
                    creation.message = error.to_string();
                }
            }
        }
        return Ok(json!({"creation":creation,"inventory":null}));
    }
    let w = workspace
        .as_mut()
        .ok_or_else(|| UiError::new("E-PROJECT-NOT-OPEN", "Open Project required"))?;
    w.detect_recovery();
    if intent.source_write() && delivery.gate.capturing() {
        return Err(UiError::new(
            "E-DELIVERY-CAPTURING",
            "saved input capture / structural operation is running; Save was not queued",
        ));
    }
    match intent {
        Intent::DeliveryStart { request } => {
            if matches!(request, crate::delivery::Request::Build { .. }) && w.recovery_required {
                return Err(UiError::new(
                    "E-RECOVERY-REQUIRED",
                    "Build is blocked until source-set Recovery is established",
                ));
            }
            delivery.start(*epoch, &w.read, request)
        }
        Intent::DeliveryState => delivery.snapshot(*epoch),
        Intent::DeliveryProblems { id, start, count } => {
            delivery.problems(*epoch, id, start, count)
        }
        Intent::DeliveryProblemTarget { id, index } => {
            let (diagnostic, captured) = delivery.problem(*epoch, id, index)?;
            let target = w.saved_problem_target(
                &diagnostic,
                &captured.config_identity,
                &captured.input_identity,
            )?;
            Ok(json!({"source":diagnostic.source,"target":target,"generation":w.generation}))
        }
        Intent::Open { .. } | Intent::CreateProject { .. } => unreachable!(),
        Intent::Inventory => Ok(inventory(w)),
        Intent::ConfigView { profile, starts } => {
            convert(w.config_view(profile.as_deref(), starts))
        }
        Intent::ConfigEdit {
            revision,
            operation,
        } => {
            w.edit_config(revision, operation)?;
            Ok(Value::Null)
        }
        Intent::ConfigSave { revision } => {
            convert(w.save_config(revision, masterdata_engine::native::Fault::None)?)
        }
        Intent::ConfigCompare { external } => {
            let (identity, before, after) = if external {
                w.configuration.compare()?
            } else {
                let (before, after) = w.configuration.saved_compare();
                (w.configuration.base.content.clone(), before, after)
            };
            Ok(
                json!({"source":"masterdata.toml","identity":identity,"before":before,"after":after,"conflict":external}),
            )
        }
        Intent::ConfigReload {
            revision,
            discard_authorized,
        } => {
            w.reload_config(revision, discard_authorized)?;
            Ok(Value::Null)
        }
        Intent::ConfigRecheck => convert(w.recheck_config()?),
        Intent::CreationChoices => Ok(w.creation_choices()),
        Intent::CreationDefaults {
            category,
            path,
            table,
        } => convert(w.creation_defaults(&category, &path, table.as_deref())?),
        Intent::CreationPreview { request } => Ok(w.creation_preview(&request)),
        Intent::CreationField { fields } => {
            convert(masterdata_engine::creation::suggest_field(&fields)?)
        }
        Intent::Create { request } => convert(w.create_source(&request)?),
        Intent::RecheckCreation { path } => convert(w.recheck_creation(&path)?),
        Intent::PathMoveChoices { source } => Ok(w.path_move_choices(&source)?),
        Intent::PathMovePreview {
            source,
            destination,
        } => convert(w.prepare_path_move(&source, &destination)?),
        Intent::MoveSource { token } => convert(w.apply_path_move(&token)?),
        Intent::RecheckMove { token } => convert(w.recheck_path_move(&token)?),
        Intent::MigrationPlan { command } => convert(w.prepare_migration(command)?),
        Intent::TableDeclarationDetail { source, table } => {
            convert(w.table_declaration_detail(&source, &table)?)
        }
        Intent::TableDeclarationPlan { command } => convert(w.prepare_table_declaration(command)?),
        Intent::TypeMigrationPlan {
            source,
            identity,
            command,
            input,
        } => convert(w.prepare_type_migration_input(&source, &identity, command, input)?),
        Intent::TypeInitializer {
            source,
            identity,
            declaration,
        } => convert(w.type_initializer_shape(&source, &identity, &declaration)?),
        Intent::FieldScope {
            source,
            revision,
            generation,
            operation,
        } => Ok(w.field_operation_scope(&source, revision, generation, operation)?),
        Intent::FieldOperation {
            source,
            revision,
            generation,
            request,
        } => convert(w.direct_field_operation(&source, revision, generation, request)?),
        Intent::MigrationCompare { token, source } => {
            convert(w.migration_compare(&token, &source)?)
        }
        Intent::MigrationApply {
            token,
            authorize_destructive,
        } => convert(w.apply_migration(&token, authorize_destructive)?),
        Intent::MigrationResult { token } => convert(w.recheck_migration_result(&token)?),
        Intent::MigrationRecovery {
            id,
            restore_old,
            authorized,
        } => convert(w.recheck_migration_recovery(&id, restore_old, authorized)?),
        Intent::SaveSource { source } => {
            convert(w.save_paths(vec![source], masterdata_engine::native::Fault::None)?)
        }
        Intent::DiscardSource { source } => {
            w.discard_source(&source)?;
            Ok(Value::Null)
        }
        Intent::AuthoringState {
            source,
            revision,
            generation,
        } => match w.check_authoring_input(&source, revision, generation) {
            Ok(()) => Ok(json!({"current":true,"reason":null})),
            Err(error) => Ok(json!({"current":false,"reason":error.to_string()})),
        },
        Intent::Select {
            path, start, count, ..
        } => {
            let mut value = convert(w.select_view(&path, start, count)?)?;
            value
                .as_object_mut()
                .unwrap()
                .insert("sessionEpoch".into(), json!(*epoch));
            Ok(value)
        }
        Intent::EditText {
            source,
            revision,
            generation,
            row,
            field,
            text,
        } => convert(w.edit_text_at(&source, revision, generation, &row, &field, &text)?),
        Intent::EditValue {
            source,
            revision,
            generation,
            row,
            path,
            value,
        } => convert(w.edit_at(&source, revision, generation, &row, &path, &value)?),
        Intent::Paste {
            source,
            revision,
            generation,
            row,
            field,
            text,
        } => convert(w.paste_at(&source, revision, generation, &row, &field, &text)?),
        Intent::Copy {
            source,
            revision,
            generation,
            anchor,
            focus,
            first,
            last,
        } => convert(w.copy_between(
            &source,
            revision,
            generation,
            [&anchor, &focus],
            [&first, &last],
        )?),
        Intent::Search { source, text } => {
            w.set_search(&source, &text)?;
            Ok(Value::Null)
        }
        Intent::Locate { source, row } => convert(w.locate_row(&source, &row)?),
        Intent::ProblemTarget {
            source,
            generation,
            occurrence,
            path,
        } => convert(w.problem_target(&source, generation, occurrence, &path)?),
        Intent::AddRow {
            source,
            revision,
            generation,
            before,
        } => convert(w.add_row(&source, revision, generation, before.as_deref())?),
        Intent::DeleteRow {
            source,
            revision,
            generation,
            row,
            restore,
        } => convert(if restore {
            w.undo_delete(&source, revision, generation, &row)?
        } else {
            w.delete_row(&source, revision, generation, &row)?
        }),
        Intent::MoveRow {
            source,
            revision,
            generation,
            row,
            before,
        } => convert(w.move_row(&source, revision, generation, &row, before.as_deref())?),
        Intent::NudgeRow {
            source,
            revision,
            generation,
            row,
            delta,
        } => convert(w.nudge_row(&source, revision, generation, &row, delta)?),
        Intent::NextRow { source, row } => convert(w.next_row(&source, &row)?),
        Intent::Columns {
            source,
            revision,
            generation,
            order,
        } => convert(w.reorder_columns_at(&source, revision, generation, &order)?),
        Intent::Complex {
            source,
            revision,
            generation,
            row,
            path,
            start,
            count,
        } => {
            let mut value = convert(w.complex_view(
                &source,
                revision,
                generation,
                &row,
                &path,
                start..start.saturating_add(count),
            )?)?;
            value
                .as_object_mut()
                .unwrap()
                .insert("sessionEpoch".into(), json!(*epoch));
            Ok(value)
        }
        Intent::ComplexEdit {
            source,
            revision,
            generation,
            row,
            path,
            operation,
        } => convert(w.complex_operation(&source, revision, generation, &row, &path, operation)?),
        Intent::Tags {
            source,
            revision,
            generation,
            row,
            start,
        } => {
            let mut value = convert(w.tag_view(&source, revision, generation, &row, start)?)?;
            value
                .as_object_mut()
                .unwrap()
                .insert("sessionEpoch".into(), json!(*epoch));
            Ok(value)
        }
        Intent::TagEdit {
            source,
            revision,
            generation,
            row,
            operation,
        } => convert(w.edit_tag(&source, revision, generation, &row, operation)?),
        Intent::Schema {
            source,
            revision,
            generation,
            field,
            nullable,
            array,
            type_name,
        } => convert(w.schema_modifier_at(
            &source,
            revision,
            generation,
            &field,
            masterdata_engine::workspace::FieldShapeEdit {
                nullable,
                array,
                type_name: type_name.as_deref(),
            },
        )?),
        Intent::Undo { source, redo } => convert(w.undo(&source, redo)?),
        Intent::Save { table, source } => convert(w.save_table(&table, source.as_deref())?),
        Intent::SaveAll => convert(w.save_all()?),
        Intent::Compare { source } => convert(w.compare(&source)?),
        Intent::Overwrite { source, identity } => convert(w.overwrite(&source, &identity)?),
        Intent::ReloadSource { source } => {
            w.reload_source(&source)?;
            Ok(Value::Null)
        }
        Intent::Problems { start, count } => Ok(
            json!({"generation":w.diagnostics_generation,"pending":w.diagnostics_pending,"total":w.diagnostics.len(),"problems":w.diagnostics.iter().skip(start).take(count.min(256)).collect::<Vec<_>>()}),
        ),
        Intent::Validate => {
            w.diagnostics_pending = true;
            Ok(Value::Null)
        }
        Intent::Remember {
            source,
            search,
            row,
            field,
            top,
            left,
        } => {
            if !w.read.sources.contains_key(&source) {
                return Err(UiError::new("E-SOURCE-MISSING", &source));
            }
            let state = w.views.entry(source).or_default();
            state.search = search;
            state.selected_row = row;
            state.selected_field = field;
            state.scroll_top = top.max(0.0);
            state.scroll_left = left.max(0.0);
            Ok(Value::Null)
        }
    }
}
