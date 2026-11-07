use crate::{events::ProgressEvent, progress_unit::ProgressUnit};
use std::{
    collections::BTreeMap,
    io::Write,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(crate) type SharedOutput = Arc<Mutex<Box<dyn Write + Send>>>;

#[derive(Default)]
pub(crate) struct ProgressState {
    pub(crate) output: Option<SharedOutput>,
    pub(crate) scope: Vec<String>,
    pub(crate) children: Vec<ProgressState>,
    pub(crate) summary: Option<String>,
    pub(crate) finished_elapsed: Option<Duration>,
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
    pub(crate) completed_label: Option<&'static str>,
    pub(crate) phase_finished: bool,
    pub(crate) done: usize,
    pub(crate) total: usize,
    pub(crate) unit: ProgressUnit,
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
    pub(crate) completed_label: Option<&'static str>,
    pub(crate) phase: &'static str,
    pub(crate) elapsed: Duration,
    pub(crate) done: usize,
    pub(crate) total: usize,
    pub(crate) unit: ProgressUnit,
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
    pub(crate) fn worker_output(&mut self, worker: usize, output: &str) {
        if let Some(state) = self.workers.get_mut(&worker) {
            for line in output.lines() {
                state.output.push_back(line.to_owned());
                while state.output.len() > 3 {
                    state.output.pop_front();
                }
            }
        }
    }

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
        self.show_with_unit(event, ProgressUnit::Count);
    }

    pub(crate) fn show_with_unit(&mut self, event: ProgressEvent, unit: ProgressUnit) {
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
                    record.completed_label = self.completed_label;
                    record.elapsed += elapsed;
                    record.done = self.done;
                    record.total = self.total;
                    record.unit = self.unit;
                    record.detail = self.phase_detail.clone();
                } else {
                    self.history.push(PhaseRecord {
                        completed_label: self.completed_label,
                        phase: self.phase,
                        elapsed,
                        done: self.done,
                        total: self.total,
                        unit: self.unit,
                        detail: self.phase_detail.clone(),
                    });
                }
            }
            if phase_changed {
                self.completed_label = None;
                self.phase_finished = false;
            }
            self.phase = event.phase;
            if phase_changed || self.phase_started.is_none() {
                self.phase_started = Some(Instant::now());
            }
            self.done = event.done;
            self.total = event.total;
            self.unit = unit;
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
        self.finished_elapsed.unwrap_or_else(|| {
            self.started
                .map_or(Duration::ZERO, |started| started.elapsed())
        })
    }

    pub(crate) fn completed_count(&self) -> usize {
        self.completed_tasks
            + self
                .children
                .iter()
                .map(Self::completed_count)
                .sum::<usize>()
    }

    pub(crate) fn active_count(&self) -> usize {
        self.workers
            .values()
            .filter(|worker| worker.status == WorkerStatus::Running)
            .count()
            + self.children.iter().map(Self::active_count).sum::<usize>()
    }

    pub(crate) fn failure_count(&self) -> usize {
        self.failed_tasks + self.children.iter().map(Self::failure_count).sum::<usize>()
    }

    pub(crate) fn last_failure(&self) -> Option<&str> {
        self.failures
            .last()
            .map(String::as_str)
            .or_else(|| self.children.iter().rev().find_map(Self::last_failure))
    }

    pub(crate) fn close_children(&mut self) {
        for child in &mut self.children {
            child.close_children();
            if !child.closed {
                child.record_failure(format!(
                    "Operation ended before completion: {}",
                    child.title
                ));
                child.finished_elapsed = Some(child.elapsed());
                child.closed = true;
                if child.plain {
                    crate::plain::finish(child, None);
                }
            }
        }
    }

    pub(crate) fn phase_elapsed(&self) -> Duration {
        let previous = self
            .history
            .iter()
            .find(|record| record.phase == self.phase)
            .map_or(Duration::ZERO, |record| record.elapsed);
        let current = self.phase_started.map_or(Duration::ZERO, |started| {
            if let (Some(elapsed), Some(operation_started)) = (self.finished_elapsed, self.started)
            {
                elapsed.saturating_sub(started.saturating_duration_since(operation_started))
            } else {
                started.elapsed()
            }
        });
        previous + current
    }
}
