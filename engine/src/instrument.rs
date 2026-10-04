//! Operation-scoped measured work counts; no adapter may supply claimed zeroes.
use serde::Serialize;
use std::{cell::RefCell, collections::BTreeMap, time::Instant};

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
    pub project_discovery: u64,
    pub project_enumeration: u64,
    pub project_yaml_parse: u64,
    pub project_validation: u64,
    pub local_parse: u64,
    pub bytes_read: u64,
}
thread_local! { static WORK:RefCell<Work>=RefCell::new(Work::default()); }
thread_local! { static STAGES:RefCell<BTreeMap<&'static str,f64>>=const { RefCell::new(BTreeMap::new()) }; }
pub struct Span {
    name: &'static str,
    start: Instant,
}
pub fn span(name: &'static str) -> Span {
    Span {
        name,
        start: Instant::now(),
    }
}
impl Drop for Span {
    fn drop(&mut self) {
        STAGES.with(|s| {
            *s.borrow_mut().entry(self.name).or_default() +=
                self.start.elapsed().as_secs_f64() * 1000.0
        });
    }
}
pub enum Kind {
    Discovery,
    Enumeration,
    ProjectParse,
    Validation,
    LocalParse,
    Bytes(u64),
}
pub fn count(kind: Kind) {
    WORK.with(|w| {
        let mut w = w.borrow_mut();
        match kind {
            Kind::Discovery => w.project_discovery += 1,
            Kind::Enumeration => w.project_enumeration += 1,
            Kind::ProjectParse => w.project_yaml_parse += 1,
            Kind::Validation => w.project_validation += 1,
            Kind::LocalParse => w.local_parse += 1,
            Kind::Bytes(n) => w.bytes_read += n,
        }
    });
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Measurement {
    pub elapsed_ms: f64,
    pub work: Work,
    pub stages_ms: BTreeMap<String, f64>,
}
pub fn measure<T>(action: impl FnOnce() -> T) -> (T, Measurement) {
    let before = WORK.with(|w| w.borrow().clone());
    let stage_before = STAGES.with(|s| s.borrow().clone());
    let start = Instant::now();
    let result = action();
    let now = WORK.with(|w| w.borrow().clone());
    let work = Work {
        project_discovery: now.project_discovery - before.project_discovery,
        project_enumeration: now.project_enumeration - before.project_enumeration,
        project_yaml_parse: now.project_yaml_parse - before.project_yaml_parse,
        project_validation: now.project_validation - before.project_validation,
        local_parse: now.local_parse - before.local_parse,
        bytes_read: now.bytes_read - before.bytes_read,
    };
    (
        result,
        Measurement {
            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
            work,
            stages_ms: STAGES.with(|s| {
                s.borrow()
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.to_string(),
                            v - stage_before.get(k).copied().unwrap_or(0.0),
                        )
                    })
                    .collect()
            }),
        },
    )
}
