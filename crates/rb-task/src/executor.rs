use crate::{ExecutionError, TaskEvent, TaskEvents, TaskGraph, TaskId};
use std::collections::{BTreeMap, VecDeque};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::time::Instant;

/// Gives an action its task and worker identity and methods to report progress.
pub struct TaskContext {
    pub task: TaskId,
    pub worker: usize,
    pub events: TaskEvents,
    started: Instant,
}

impl TaskContext {
    pub fn output(&self, line: impl Into<String>) {
        self.events.emit(TaskEvent::Output {
            task: self.task,
            worker: self.worker,
            line: line.into(),
            elapsed: self.started.elapsed(),
        });
    }

    pub fn progress(&self, phase: &'static str, done: usize, total: usize, detail: Option<String>) {
        self.events.emit(TaskEvent::Progress {
            task: self.task,
            worker: self.worker,
            phase,
            done,
            total,
            detail,
            elapsed: self.started.elapsed(),
        });
    }
}

/// Holds the caller's executable work, returning an error message on failure.
pub type TaskAction = Arc<dyn Fn(TaskContext) -> Result<(), String> + Send + Sync + 'static>;

/// Runs ready tasks within a worker limit and waits for active work before returning.
pub struct Executor {
    workers: usize,
    events: TaskEvents,
}

impl Executor {
    pub fn new(workers: usize, events: TaskEvents) -> Self {
        Self {
            workers: workers.max(1),
            events,
        }
    }

    pub fn run(
        &self,
        graph: TaskGraph,
        actions: BTreeMap<TaskId, TaskAction>,
    ) -> Result<(), ExecutionError> {
        if actions.len() != graph.len() || actions.keys().any(|id| graph.task(*id).is_none()) {
            return Err(ExecutionError::InvalidActions);
        }
        let mut schedule = graph.schedule().map_err(ExecutionError::InvalidGraph)?;
        let count = self.workers.min(graph.len());
        thread::scope(|scope| {
            let (completed, completions) = std::sync::mpsc::channel();
            let mut senders = Vec::new();
            for worker in 0..count {
                let (sender, receiver) = std::sync::mpsc::channel::<(TaskId, String, TaskAction)>();
                senders.push(sender);
                let completed = completed.clone();
                let events = self.events.clone();
                scope.spawn(move || {
                    while let Ok((task, label, action)) = receiver.recv() {
                        let started = Instant::now();
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            events.emit(TaskEvent::Started {
                                task,
                                worker,
                                label,
                                elapsed: Duration::ZERO,
                            });
                            action(TaskContext {
                                task,
                                worker,
                                events: events.clone(),
                                started,
                            })
                        }))
                        .map_err(|_| ExecutionError::Panicked { task })
                        .and_then(|result| {
                            result.map_err(|message| ExecutionError::TaskFailed { task, message })
                        });
                        let terminal = match &result {
                            Ok(()) => TaskEvent::Finished {
                                task,
                                worker,
                                elapsed: started.elapsed(),
                            },
                            Err(error) => TaskEvent::Failed {
                                task,
                                worker,
                                error: error.to_string(),
                                elapsed: started.elapsed(),
                            },
                        };
                        let notification =
                            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                events.emit(terminal)
                            }));
                        let result = if notification.is_err() {
                            Err(ExecutionError::Panicked { task })
                        } else {
                            result
                        };
                        if completed.send((task, worker, result)).is_err() {
                            break;
                        }
                    }
                });
            }
            drop(completed);
            let mut available: VecDeque<_> = (0..count).collect();
            let mut active = 0;
            let mut failure = None;
            loop {
                while failure.is_none() && !available.is_empty() {
                    let Some(task) = schedule.take_ready() else {
                        break;
                    };
                    let worker = available.pop_front().unwrap();
                    senders[worker]
                        .send((
                            task,
                            schedule.task(task).name.clone(),
                            actions[&task].clone(),
                        ))
                        .unwrap();
                    active += 1;
                }
                if active == 0 {
                    break;
                }
                let (task, worker, result) = completions.recv().unwrap();
                active -= 1;
                available.push_back(worker);
                match result {
                    Ok(()) => {
                        if let Err(error) = schedule.complete(task) {
                            failure.get_or_insert(ExecutionError::InvalidCompletion(error));
                        }
                    }
                    Err(error) => {
                        failure.get_or_insert(error);
                    }
                }
            }
            drop(senders);
            failure.map_or(Ok(()), Err)
        })
    }
}

#[cfg(test)]
mod tests;
