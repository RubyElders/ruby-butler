use std::{
    io::{self, IsTerminal, Write},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::{Duration, Instant},
};

#[cfg(feature = "rb-task")]
use crate::task::TaskBridge;
#[cfg(feature = "rb-task")]
use rb_task::TaskEventSink;

use crate::{events::ProgressEvent, render, state::ProgressState};

#[derive(Clone)]
pub struct ProgressHandle {
    progress: Arc<Mutex<ProgressState>>,
}

impl ProgressHandle {
    pub fn emit(&self, event: ProgressEvent) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        if progress.plain {
            crate::plain::progress(&progress, &event);
        }
        progress.show(event);
        render::draw(&mut progress);
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
        render::draw(&mut progress);
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
        render::draw(&mut progress);
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
        render::draw(&mut progress);
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
        render::draw(&mut progress);
    }

    pub fn suspend(&self) {
        let mut progress = self.progress.lock().unwrap();
        if progress.closed {
            return;
        }
        progress.external_output = true;
        render::draw(&mut progress);
    }
}

pub struct Reporter {
    progress: Arc<Mutex<ProgressState>>,
    running: Arc<AtomicBool>,
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
        let progress = Arc::new(Mutex::new(ProgressState {
            plain,
            title,
            output: Some(Mutex::new(Box::new(io::BufWriter::new(writer)))),
            started: Some(Instant::now()),
            ..Default::default()
        }));
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
                    render::draw(&mut ticker_progress.lock().unwrap());
                    thread::sleep(Duration::from_millis(250));
                }
            })
        });
        Self {
            progress,
            running,
            ticker,
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

    fn stop(&mut self, summary: Option<String>, compact: bool) {
        if self.finished {
            return;
        }
        self.finished = true;
        self.running.store(false, Ordering::Relaxed);
        if let Some(ticker) = self.ticker.take() {
            let _ = ticker.join();
        }
        let mut progress = self.progress.lock().unwrap();
        if compact && progress.failed_tasks == 0 && !progress.external_output {
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
mod tests;
