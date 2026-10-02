use crate::{
    events::ProgressEvent,
    state::{ProgressState, WorkerStatus},
    terminal::format_duration,
};
use rb_task::{TaskEvent, TaskEventSink};
use std::sync::{Arc, Mutex};

pub(crate) struct TaskBridge {
    pub(crate) progress: Arc<Mutex<ProgressState>>,
}

impl TaskEventSink for TaskBridge {
    fn event(&self, event: TaskEvent) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        if progress.plain {
            write_plain_event(&progress, &event);
        }
        if let TaskEvent::Failed {
            task,
            worker,
            error,
            ..
        } = &event
        {
            progress.record_failure(format!("{error} (worker {worker}, task {})", task.index()));
        }
        let transition = match &event {
            TaskEvent::Started { worker, .. } => Some((*worker, WorkerStatus::Running)),
            TaskEvent::Finished { worker, .. } => Some((*worker, WorkerStatus::Succeeded)),
            TaskEvent::Failed { worker, .. } => Some((*worker, WorkerStatus::Failed)),
            _ => None,
        };
        let event = match event {
            TaskEvent::Started {
                task,
                worker,
                label,
                elapsed,
            } => ProgressEvent {
                worker: Some(worker),
                phase: "running",
                done: 0,
                total: 1,
                detail: Some(format!("{}: {}", task.index(), label)),
                elapsed: Some(elapsed),
            },
            TaskEvent::Output { worker, line, .. } => {
                progress.worker_output(worker, &line);
                crate::render::draw(&mut progress);
                return;
            }
            TaskEvent::Progress {
                worker,
                phase,
                done,
                total,
                detail,
                elapsed,
                ..
            } => ProgressEvent {
                worker: Some(worker),
                phase,
                done,
                total,
                detail,
                elapsed: Some(elapsed),
            },
            TaskEvent::Finished {
                task,
                worker,
                elapsed,
            } => ProgressEvent {
                worker: Some(worker),
                phase: "done",
                done: 1,
                total: 1,
                detail: Some(format!("{}: complete", task.index())),
                elapsed: Some(elapsed),
            },
            TaskEvent::Failed {
                task,
                worker,
                error,
                elapsed,
            } => ProgressEvent {
                worker: Some(worker),
                phase: "failed",
                done: 1,
                total: 1,
                detail: Some(format!("{}: {error}", task.index())),
                elapsed: Some(elapsed),
            },
        };
        let elapsed = event.elapsed;
        progress.show(event);
        if let Some((worker, status)) = transition {
            progress.set_worker_status(worker, status);
            progress.workers.get_mut(&worker).unwrap().elapsed = elapsed;
        }
        crate::render::draw(&mut progress);
    }
}
fn write_plain_event(state: &ProgressState, event: &TaskEvent) {
    let (worker, task, message) = match event {
        TaskEvent::Started {
            worker,
            task,
            label,
            ..
        } => (worker, task, format!("started: {label}")),
        TaskEvent::Output {
            worker, task, line, ..
        } => (worker, task, line.clone()),
        TaskEvent::Progress {
            worker,
            task,
            phase,
            done,
            total,
            detail,
            ..
        } => (
            worker,
            task,
            format!("{phase} {done}/{total} {}", detail.as_deref().unwrap_or("")),
        ),
        TaskEvent::Finished {
            worker,
            task,
            elapsed,
        } => (
            worker,
            task,
            format!("complete ({})", format_duration(*elapsed)),
        ),
        TaskEvent::Failed {
            worker,
            task,
            elapsed,
            error,
        } => (
            worker,
            task,
            format!("failed: {error} ({})", format_duration(*elapsed)),
        ),
    };
    crate::plain::line(
        state,
        &format!("worker: {worker} | task: {}", task.index()),
        &message,
    );
}
