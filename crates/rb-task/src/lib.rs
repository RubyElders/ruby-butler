#![doc = include_str!("../README.md")]

mod error;
mod events;
mod executor;
mod graph;

pub use error::ExecutionError;
pub use events::{TaskEvent, TaskEventSink, TaskEvents};
pub use executor::{Executor, TaskAction, TaskContext};
pub use graph::{CompletionError, PlanError, Schedule, Task, TaskGraph};

/// Identifies a task within its graph; IDs must not be mixed between graphs.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TaskId(usize);

impl TaskId {
    pub fn index(self) -> usize {
        self.0
    }
}
