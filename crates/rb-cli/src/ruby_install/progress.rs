use crate::reporting::Completion;
use anyhow::Result;
use rb_progress::{ProgressHandle, Reporter};
use rb_ruby_installer::{Event, Installation, Phase};
use semver::Version;
use std::time::{Duration, Instant};

const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);

pub(super) struct InstallProgress {
    reporter: Reporter,
    handle: ProgressHandle,
    phase: Option<Phase>,
    last_download: Instant,
}

impl InstallProgress {
    pub(super) fn new(reporter: Reporter) -> Self {
        Self {
            handle: reporter.handle(),
            reporter,
            phase: None,
            last_download: Instant::now(),
        }
    }

    pub(super) fn event(&mut self, event: Event) {
        match event {
            Event::Phase(next) => {
                self.complete_phase();
                self.phase = Some(next);
                self.handle
                    .phase_started(next.label(), next.completed_label());
                if next == Phase::Downloading {
                    self.handle.byte_progress(next.label(), 0, 0);
                }
            }
            Event::Selected { version } => {
                self.handle.phase_finished(
                    Phase::Selecting.label(),
                    Phase::Selecting.completed_label(),
                    Some(version.to_string()),
                );
                self.phase = None;
            }
            Event::Download { received, total } => {
                if received == total || self.last_download.elapsed() >= PROGRESS_INTERVAL {
                    self.handle.byte_progress(
                        Phase::Downloading.label(),
                        received as usize,
                        total as usize,
                    );
                    self.last_download = Instant::now();
                }
            }
        }
    }

    fn complete_phase(&self) {
        if let Some(phase) = self.phase {
            self.handle
                .phase_finished(phase.label(), phase.completed_label(), None);
        }
    }

    pub(super) fn finish(
        self,
        result: Result<Installation>,
        completion: Completion,
    ) -> Result<Version> {
        match result {
            Ok(installation) => {
                self.complete_phase();
                let outcome = if installation.installed {
                    format!("Installed Ruby {}", installation.version)
                } else {
                    format!("Ruby {} is already installed", installation.version)
                };
                let summary = format!("{outcome} in {}", installation.path.display());
                completion.finish(self.reporter, summary);
                Ok(installation.version)
            }
            Err(error) => {
                self.handle.fail(format!("{error:#}"));
                self.reporter.finish();
                Err(error)
            }
        }
    }
}
