//! Native session scheduling. Queue locks only protect pending work, never parsing or I/O.
use masterdata_engine::{
    Error, instrument,
    project::{Diagnostic, Project},
    source::Value as SourceValue,
    workspace::Workspace,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    path::Path,
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
    fn new(code: &str, message: &str) -> Self {
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
    Open {
        path: String,
        discard: bool,
    },
    Inventory,
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
    stopped: bool,
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
    pub recovery_required: bool,
    pub diagnostics_pending: bool,
    pub problem_count: usize,
    pub uncertain: Vec<String>,
}
#[derive(Clone)]
pub struct Session {
    queue: Arc<Queue>,
    pub protected: Arc<AtomicBool>,
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
        let q = queue.clone();
        let guard = protected.clone();
        thread::Builder::new().name("masterdata-workspace".into()).spawn(move||{
   let mut workspace:Option<Workspace>=None;let mut epoch=0;let mut scheduled=0;
   loop{
    let job={let mut p=q.pending.lock().unwrap();
     while p.commands.is_empty()&&p.selection.is_none()&&p.diagnostics.is_none()&&!p.stopped{p=q.wake.wait(p).unwrap();}
     if p.stopped{break;}
     if let Some(result)=p.diagnostics.take(){drop(p);if epoch==result.epoch&&let Some(w)=workspace.as_mut(){w.accept_diagnostics(result.generation,result.problems);}None}
     else {p.commands.pop_front().or_else(||p.selection.take())}
    };
    if let Some(job)=job{
     if job.epoch.is_some_and(|expected|expected!=epoch){let _=job.reply.send(Err(UiError::new("E-PROJECT-OBSOLETE","request belongs to a previous Project session")));continue;}
     let token=job.intent.token();
     if token.is_some_and(|t|t!=q.latest.load(Ordering::Acquire)){let _=job.reply.send(Err(UiError::new("E-SELECTION-OBSOLETE","selection superseded")));continue;}
     let queued_ms=job.enqueued.elapsed().as_secs_f64()*1000.0;let start=Instant::now();
     let previous_epoch=epoch;
     if matches!(job.intent,Intent::Validate){scheduled=0;}
     let (result,measurement)=instrument::measure(||execute(&mut workspace,&mut epoch,job.intent));
     if epoch!=previous_epoch{scheduled=0;}
     guard.store(workspace.as_ref().is_some_and(|w|!w.dirty_paths().is_empty()||w.recovery_required||!w.uncertain_paths().is_empty()),Ordering::Release);
     let reply=if token.is_some_and(|t|t!=q.latest.load(Ordering::Acquire)){Err(UiError::new("E-SELECTION-OBSOLETE","selection superseded"))}else{
      result.and_then(|data|{let encode=Instant::now();let encoded=serde_json::to_string(&data).map_err(|e|UiError::new("E-IPC",&e.to_string()))?;
       let serialization_ms=encode.elapsed().as_secs_f64()*1000.0;let meta=json!({"epoch":epoch,"queuedMs":queued_ms,"backendMs":measurement.elapsed_ms,"serializationMs":serialization_ms,"bytes":encoded.len(),"token":token,"work":measurement.work,"stagesMs":measurement.stages_ms,"nativeCompleteMs":start.elapsed().as_secs_f64()*1000.0});
       Ok(format!("{{\"data\":{encoded},\"host\":{meta}}}"))
      })
     };
     let _=job.reply.send(reply);
    }
    if let Some(w)=workspace.as_ref(){
     guard.store(!w.dirty_paths().is_empty()||w.recovery_required||!w.uncertain_paths().is_empty(),Ordering::Release);
     publish(Status{open:true,epoch,generation:w.generation,dirty:w.dirty_paths(),recovery_required:w.recovery_required,diagnostics_pending:w.diagnostics_pending,problem_count:w.diagnostics.len(),uncertain:w.uncertain_paths()});
     if w.diagnostics_pending&&scheduled!=w.generation{
      let mut p=validator.pending.lock().unwrap();p.snapshot=Some((epoch,w.diagnostic_input()));scheduled=w.generation;validator.wake.notify_one();
     }
    }
   }
   let mut p=validator.pending.lock().unwrap();p.stopped=true;validator.wake.notify_one();
  }).expect("workspace worker");
        Self { queue, protected }
    }
    pub fn submit(&self, intent: Intent) -> oneshot::Receiver<Reply> {
        self.submit_at(intent, None)
    }
    fn submit_at(&self, intent: Intent, epoch: Option<u64>) -> oneshot::Receiver<Reply> {
        let (tx, rx) = oneshot::channel();
        let mut pending = self.queue.pending.lock().unwrap();
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
}
fn convert<T: Serialize>(v: T) -> Result<Value, UiError> {
    serde_json::to_value(v).map_err(|e| UiError::new("E-IPC", &e.to_string()))
}
fn inventory(w: &Workspace) -> Value {
    json!({"project":w.read.config.project,"root":w.read.root,"roots":w.read.config.sources.roots,"sources":w.read.sources.values().map(|s|json!({"path":s.path,"kind":s.kind,"binding":s.binding,"error":s.error})).collect::<Vec<_>>(),"types":w.read.types.keys().collect::<Vec<_>>(),"dirty":w.dirty_paths(),"uncertain":w.uncertain_paths(),"generation":w.generation})
}
fn execute(
    workspace: &mut Option<Workspace>,
    epoch: &mut u64,
    intent: Intent,
) -> Result<Value, UiError> {
    if let Intent::Open { path, discard } = intent {
        if workspace
            .as_ref()
            .is_some_and(|w| !w.dirty_paths().is_empty())
            && !discard
        {
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
    let w = workspace
        .as_mut()
        .ok_or_else(|| UiError::new("E-PROJECT-NOT-OPEN", "Open Project required"))?;
    match intent {
        Intent::Open { .. } => unreachable!(),
        Intent::Inventory => Ok(inventory(w)),
        Intent::Select {
            path, start, count, ..
        } => {
            let mut value = convert(w.select(&path, start, count)?)?;
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
        } => {
            if w.generation != generation {
                return Err(Error::new("E-DRAFT-STALE", "semantic generation changed").into());
            }
            convert(w.edit(&source, revision, &row, &path, &value)?)
        }
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
        Intent::Compare { source } => {
            let (identity, before, after) = w.compare(&source)?;
            Ok(json!({"source":source,"identity":identity,"before":before,"after":after}))
        }
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
