use std::{
    io::{self, IsTerminal, Write},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(feature = "rb-task")]
use crate::task::TaskBridge;
#[cfg(feature = "rb-task")]
use rb_task::TaskEventSink;

use crate::{
    events::ProgressEvent, progress_unit::ProgressUnit, render, state::ProgressState,
    target::ProgressTarget,
};

#[derive(Clone)]
pub struct ProgressHandle {
    progress: ProgressTarget,
}

impl ProgressHandle {
    pub fn emit(&self, event: ProgressEvent) {
        self.emit_with_unit(event, ProgressUnit::Count);
    }

    pub fn phase_started(&self, active: &'static str, completed: &'static str) {
        self.phase_status(active, completed, false, None);
    }

    pub fn phase_finished(
        &self,
        active: &'static str,
        completed: &'static str,
        detail: Option<String>,
    ) {
        self.phase_status(active, completed, true, detail);
    }

    fn phase_status(
        &self,
        active: &'static str,
        completed: &'static str,
        finished: bool,
        detail: Option<String>,
    ) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        let same_phase = progress.phase == active;
        let (unit, done, total) = if same_phase {
            (progress.unit, progress.done, progress.total)
        } else {
            (ProgressUnit::Status, 0, 0)
        };
        let event = ProgressEvent {
            worker: None,
            phase: active,
            done,
            total,
            detail,
            elapsed: None,
        };
        progress.show_with_unit(event.clone(), unit);
        progress.completed_label = Some(completed);
        progress.phase_finished = finished;
        if progress.plain {
            crate::plain::progress(&progress, &event, unit);
        }
        progress.draw();
    }

    pub fn byte_progress(&self, phase: &'static str, received: usize, total: usize) {
        self.emit_with_unit(
            ProgressEvent {
                worker: None,
                phase,
                done: received,
                total,
                detail: None,
                elapsed: None,
            },
            ProgressUnit::Bytes,
        );
    }

    fn emit_with_unit(&self, event: ProgressEvent, unit: ProgressUnit) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        progress.show_with_unit(event.clone(), unit);
        if progress.plain {
            crate::plain::progress(&progress, &event, unit);
        }
        progress.draw();
    }

    pub fn event(&self, phase: &'static str, done: usize, total: usize, detail: Option<String>) {
        self.worker_event(None, phase, done, total, detail);
    }

    pub fn worker_event(
        &self,
        worker: Option<usize>,
        phase: &'static str,
        done: usize,
        total: usize,
        detail: Option<String>,
    ) {
        self.emit(ProgressEvent {
            worker,
            phase,
            done,
            total,
            detail,
            elapsed: None,
        });
    }

    pub fn worker_started(&self, worker: usize, label: impl Into<String>) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        let label = label.into();
        if progress.plain {
            crate::plain::line(
                &progress,
                &format!("worker: {worker}"),
                &format!("started: {label}"),
            );
        }
        progress.show(ProgressEvent {
            worker: Some(worker),
            phase: "running",
            done: 0,
            total: 1,
            detail: Some(label),
            elapsed: None,
        });
        progress.set_worker_status(worker, crate::state::WorkerStatus::Running);
        progress.draw();
    }

    /// Appends output to an existing worker's three-line terminal preview.
    /// Plain reporters write every line with its worker prefix.
    pub fn worker_output(&self, worker: usize, output: &str) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        if progress.plain {
            crate::plain::line(&progress, &format!("worker: {worker}"), output);
        }
        progress.worker_output(worker, output);
        progress.draw();
    }

    pub fn worker_finished(&self, worker: usize) {
        self.end_worker(worker, None);
    }

    pub fn worker_failed(&self, worker: usize, error: impl Into<String>) {
        self.end_worker(worker, Some(error.into()));
    }

    fn end_worker(&self, worker: usize, error: Option<String>) {
        let mut progress = self.progress.lock().unwrap();
        let status = if error.is_some() {
            crate::state::WorkerStatus::Failed
        } else {
            crate::state::WorkerStatus::Succeeded
        };
        if progress.closed || !progress.set_worker_status(worker, status) {
            return;
        }
        let message = if let Some(error) = error {
            progress.workers.get_mut(&worker).unwrap().detail = Some(error.clone());
            progress.record_failure(format!("{error} (worker {worker})"));
            format!("failed: {error}")
        } else {
            "complete".into()
        };
        if progress.plain {
            crate::plain::line(&progress, &format!("worker: {worker}"), &message);
        }
        progress.draw();
    }

    pub fn fail(&self, error: impl Into<String>) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        let error = error.into();
        if progress.plain {
            crate::plain::line(&progress, "overall", &format!("failed: {error}"));
        }
        progress.record_failure(error);
        progress.draw();
    }

    pub fn suspend(&self) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        progress.suspend();
    }
}

pub struct Reporter {
    progress: ProgressTarget,
    running: Option<Arc<AtomicBool>>,
    ticker: Option<thread::JoinHandle<()>>,
    finished: bool,
}

impl Reporter {
    pub fn start(title: impl Into<String>) -> Option<Self> {
        if !io::stderr().is_terminal() {
            return Some(Self::with_plain_writer(title, io::stderr()));
        }
        Some(Self::with_writer(title, io::stderr()))
    }

    pub fn with_writer(title: impl Into<String>, writer: impl Write + Send + 'static) -> Self {
        Self::build(title.into(), writer, false)
    }

    pub fn with_plain_writer(
        title: impl Into<String>,
        writer: impl Write + Send + 'static,
    ) -> Self {
        Self::build(title.into(), writer, true)
    }

    fn build(title: String, writer: impl Write + Send + 'static, plain: bool) -> Self {
        let progress = ProgressTarget::new(ProgressState {
            plain,
            title,
            output: Some(Arc::new(std::sync::Mutex::new(Box::new(
                io::BufWriter::new(writer),
            )))),
            started: Some(Instant::now()),
            ..Default::default()
        });
        let running = Arc::new(AtomicBool::new(true));
        let ticker_progress = progress.clone();
        let ticker_running = running.clone();
        if plain {
            let state = progress.lock().unwrap();
            crate::plain::line(&state, "overall", &state.title);
        }
        let ticker = (!plain).then(|| {
            thread::spawn(move || {
                while ticker_running.load(Ordering::Relaxed) {
                    ticker_progress.lock().unwrap().draw();
                    thread::sleep(Duration::from_millis(250));
                }
            })
        });
        Self {
            progress,
            running: Some(running),
            ticker,
            finished: false,
        }
    }

    pub fn child(&self, title: impl Into<String>) -> Self {
        Self {
            progress: self.progress.child(title.into()),
            running: None,
            ticker: None,
            finished: false,
        }
    }

    pub fn finish(mut self) {
        self.stop(None, false);
    }

    pub fn finish_with_summary(mut self, summary: impl Into<String>) {
        self.stop(Some(summary.into()), false);
    }

    pub fn finish_compact(mut self, summary: impl Into<String>) {
        self.stop(Some(summary.into()), true);
    }

    pub fn handle(&self) -> ProgressHandle {
        ProgressHandle {
            progress: self.progress.clone(),
        }
    }

    #[cfg(feature = "rb-task")]
    pub fn task_sink(&self) -> Arc<dyn TaskEventSink> {
        Arc::new(TaskBridge {
            progress: self.progress.clone(),
        })
    }

    fn stop(&mut self, mut summary: Option<String>, compact: bool) {
        if self.finished {
            return;
        }
        self.finished = true;
        if let Some(running) = &self.running {
            running.store(false, Ordering::Relaxed);
        }
        if let Some(ticker) = self.ticker.take() {
            let _ = ticker.join();
        }
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        progress.close_children();
        progress.finished_elapsed = Some(progress.elapsed());
        if !progress.is_root() {
            progress.summary = summary;
            progress.closed = true;
            if progress.plain {
                crate::plain::finish(&progress, progress.summary.as_deref());
            }
            progress.draw();
            return;
        }
        if progress.failed_tasks == 0 && progress.failure_count() > 0 {
            summary = None;
        }
        if compact && progress.failure_count() == 0 && !progress.external_output {
            render::finish_compact(&mut progress, summary);
        } else {
            render::finish(&mut progress, summary);
        }
        progress.closed = true;
    }
}

impl Drop for Reporter {
    fn drop(&mut self) {
        self.stop(None, false);
    }
}

#[cfg(test)]
use std::sync::Mutex;

#[cfg(test)]
mod tests;
