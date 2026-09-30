//! Opt-in read-path instrumentation shared by the native benchmark and Desktop.
//! Durations are inclusive; nested phases must not be summed as a wall time.
use serde::Serialize;
use std::{cell::RefCell, collections::BTreeMap, time::Instant};

#[derive(Clone, Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadPhase {
    pub calls: u64,
    pub ms: f64,
}
#[derive(Clone, Default, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadMetrics {
    pub phases: BTreeMap<&'static str, ReadPhase>,
}
thread_local! { static METRICS: RefCell<Option<ReadMetrics>> = const { RefCell::new(None) }; }

pub struct ReadSpan {
    name: &'static str,
    started: Option<Instant>,
}
pub fn read_span(name: &'static str) -> ReadSpan {
    ReadSpan {
        name,
        started: METRICS.with(|metrics| metrics.borrow().as_ref().map(|_| Instant::now())),
    }
}
impl Drop for ReadSpan {
    fn drop(&mut self) {
        if let Some(started) = self.started {
            METRICS.with(|metrics| {
                if let Some(metrics) = metrics.borrow_mut().as_mut() {
                    let phase = metrics.phases.entry(self.name).or_default();
                    phase.calls += 1;
                    phase.ms += started.elapsed().as_secs_f64() * 1000.0;
                }
            });
        }
    }
}
pub fn measure_read<T>(operation: impl FnOnce() -> T) -> (T, ReadMetrics) {
    struct Restore(Option<ReadMetrics>);
    impl Drop for Restore {
        fn drop(&mut self) {
            METRICS.with(|metrics| *metrics.borrow_mut() = self.0.take());
        }
    }
    let restore = Restore(METRICS.with(|metrics| metrics.replace(Some(ReadMetrics::default()))));
    let result = operation();
    let metrics = METRICS.with(|metrics| metrics.borrow_mut().take().unwrap_or_default());
    drop(restore);
    (result, metrics)
}
