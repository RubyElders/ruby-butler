use std::time::Duration;

#[derive(Clone, Debug)]
pub struct ProgressEvent {
    pub worker: Option<usize>,
    pub phase: &'static str,
    pub done: usize,
    pub total: usize,
    pub detail: Option<String>,
    pub elapsed: Option<Duration>,
}
