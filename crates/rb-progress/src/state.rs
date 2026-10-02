use crate::events::ProgressEvent;
use std::{
    collections::BTreeMap,
    io::Write,
    sync::Mutex,
    time::{Duration, Instant},
};

#[derive(Default)]
pub(crate) struct ProgressState {
    pub(crate) output: Option<Mutex<Box<dyn Write + Send>>>,
    pub(crate) closed: bool,
    pub(crate) plain: bool,
    pub(crate) failures: Vec<String>,
    pub(crate) completed_tasks: usize,
    pub(crate) failed_tasks: usize,
    pub(crate) title: String,
    pub(crate) started: Option<Instant>,
    pub(crate) phase: &'static str,
    pub(crate) phase_started: Option<Instant>,
    pub(crate) phase_detail: Option<String>,
    pub(crate) done: usize,
    pub(crate) total: usize,
    pub(crate) workers: BTreeMap<usize, WorkerProgress>,
    pub(crate) history: Vec<PhaseRecord>,
    pub(crate) rendered_lines: usize,
    pub(crate) frame_capacity: usize,
    pub(crate) anchor_started: bool,
    pub(crate) external_output: bool,
    pub(crate) handoff_drawn: bool,
    pub(crate) handoff_phase: Option<&'static str>,
    pub(crate) handoff_tail_rows: usize,
}

pub(crate) struct PhaseRecord {
    pub(crate) phase: &'static str,
    pub(crate) elapsed: Duration,
    pub(crate) done: usize,
    pub(crate) total: usize,
    pub(crate) detail: Option<String>,
}

#[derive(Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum WorkerStatus {
    #[default]
    Running,
    Succeeded,
    Failed,
}

pub(crate) struct WorkerProgress {
    pub(crate) status: WorkerStatus,
    pub(crate) phase: &'static str,
    pub(crate) detail: Option<String>,
    pub(crate) started: Instant,
    pub(crate) elapsed: Option<Duration>,
    pub(crate) output: std::collections::VecDeque<String>,
}

impl ProgressState {
    pub(crate) fn set_worker_status(&mut self, worker: usize, status: WorkerStatus) -> bool {
        let Some(state) = self.workers.get_mut(&worker) else {
            return false;
        };
        if status != WorkerStatus::Running && state.status != WorkerStatus::Running {
            return false;
        }
        if status == WorkerStatus::Running {
            state.started = Instant::now();
            state.elapsed = None;
            state.output.clear();
        } else {
            state.elapsed = Some(state.elapsed.unwrap_or_default() + state.started.elapsed());
        }
        state.status = status;
        if status == WorkerStatus::Succeeded {
            self.completed_tasks += 1;
        }
        true
    }

    pub(crate) fn record_failure(&mut self, error: String) {
        self.failed_tasks += 1;
        self.failures.push(error);
    }

    pub(crate) fn show(&mut self, event: ProgressEvent) {
        self.started.get_or_insert_with(Instant::now);
        if event.worker.is_none() {
            let phase_changed = self.phase != event.phase;
            if !self.phase.is_empty() && phase_changed {
                let elapsed = self
                    .phase_started
                    .map_or(Duration::ZERO, |started| started.elapsed());
                if let Some(record) = self
                    .history
                    .iter_mut()
                    .find(|record| record.phase == self.phase)
                {
                    record.elapsed += elapsed;
                    record.done = self.done;
                    record.total = self.total;
                    record.detail = self.phase_detail.clone();
                } else {
                    self.history.push(PhaseRecord {
                        phase: self.phase,
                        elapsed,
                        done: self.done,
                        total: self.total,
                        detail: self.phase_detail.clone(),
                    });
                }
            }
            self.phase = event.phase;
            if phase_changed || self.phase_started.is_none() {
                self.phase_started = Some(Instant::now());
            }
            self.done = event.done;
            self.total = event.total;
            self.phase_detail = event.detail.clone();
            if !self.external_output {
                self.handoff_drawn = false;
            }
        }
        if let Some(worker) = event.worker {
            let started = if event.elapsed.is_none() {
                self.workers
                    .get(&worker)
                    .map_or_else(Instant::now, |state| state.started)
            } else {
                Instant::now()
            };
            self.workers.insert(
                worker,
                WorkerProgress {
                    status: self
                        .workers
                        .get(&worker)
                        .map_or(WorkerStatus::Running, |state| state.status),
                    phase: event.phase,
                    detail: event.detail,
                    started,
                    elapsed: event.elapsed,
                    output: self
                        .workers
                        .get(&worker)
                        .map(|state| state.output.clone())
                        .unwrap_or_default(),
                },
            );
        }
    }

    pub(crate) fn elapsed(&self) -> Duration {
        self.started
            .map_or(Duration::ZERO, |started| started.elapsed())
    }

    pub(crate) fn phase_elapsed(&self) -> Duration {
        self.phase_started
            .map_or(Duration::ZERO, |started| started.elapsed())
    }
}
