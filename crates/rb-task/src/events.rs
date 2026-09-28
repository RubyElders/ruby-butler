use crate::TaskId;
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// Reports task lifecycle changes, output, and progress to observers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TaskEvent {
    Started {
        task: TaskId,
        worker: usize,
        label: String,
        elapsed: Duration,
    },
    Output {
        task: TaskId,
        worker: usize,
        line: String,
        elapsed: Duration,
    },
    Progress {
        task: TaskId,
        worker: usize,
        phase: &'static str,
        done: usize,
        total: usize,
        detail: Option<String>,
        elapsed: Duration,
    },
    Finished {
        task: TaskId,
        worker: usize,
        elapsed: Duration,
    },
    Failed {
        task: TaskId,
        worker: usize,
        error: String,
        elapsed: Duration,
    },
}

/// Receives events synchronously on workers; callbacks may run concurrently.
pub trait TaskEventSink: Send + Sync {
    fn event(&self, event: TaskEvent);
}

/// Shares subscriptions and delivers each event to registered observers.
#[derive(Default, Clone)]
pub struct TaskEvents {
    sinks: Arc<Mutex<Vec<Arc<dyn TaskEventSink>>>>,
}

impl TaskEvents {
    pub fn subscribe(&self, sink: Arc<dyn TaskEventSink>) {
        self.sinks.lock().unwrap().push(sink);
    }

    pub fn emit(&self, event: TaskEvent) {
        let sinks = self.sinks.lock().unwrap().clone();
        for sink in sinks {
            sink.event(event.clone());
        }
    }
}

#[cfg(test)]
mod tests;
