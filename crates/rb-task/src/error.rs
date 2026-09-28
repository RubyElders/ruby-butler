use crate::{CompletionError, PlanError, TaskId};
use std::{error::Error, fmt};

/// Identifies a validation, task, or panic failure encountered during execution.
#[derive(Debug, Eq, PartialEq)]
pub enum ExecutionError {
    InvalidActions,
    InvalidGraph(PlanError),
    InvalidCompletion(CompletionError),
    TaskFailed { task: TaskId, message: String },
    Panicked { task: TaskId },
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidActions => formatter.write_str("missing or invalid task action"),
            Self::InvalidGraph(error) => write!(formatter, "invalid task graph: {error}"),
            Self::InvalidCompletion(error) => write!(formatter, "invalid task completion: {error}"),
            Self::TaskFailed { task, message } => {
                write!(formatter, "task {} failed: {message}", task.index())
            }
            Self::Panicked { task } => {
                write!(formatter, "task {} or its observer panicked", task.index())
            }
        }
    }
}

impl Error for ExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InvalidGraph(error) => Some(error),
            Self::InvalidCompletion(error) => Some(error),
            _ => None,
        }
    }
}
