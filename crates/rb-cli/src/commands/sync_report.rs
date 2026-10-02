use rb_core::bundler::{SyncEvent, SyncResult};
use rb_core::butler::ButlerError;
use rb_progress::Reporter;
use std::io::IsTerminal;

pub(super) fn sync_reporter() -> Reporter {
    if std::io::stderr().is_terminal() && std::io::stdout().is_terminal() {
        Reporter::with_writer("Preparing your bundle", std::io::stderr())
    } else {
        Reporter::with_plain_writer("Preparing your bundle", std::io::stderr())
    }
}

pub(super) enum Completion {
    Tree,
    Compact,
}

pub(super) fn report_sync(
    reporter: Reporter,
    completion: Completion,
    run: impl FnOnce(&mut dyn FnMut(SyncEvent<'_>)) -> std::io::Result<SyncResult>,
) -> Result<(), ButlerError> {
    let progress = reporter.handle();
    progress.worker_started(0, "Bundler");
    let mut active_phase = None;
    let mut active_detail = None;
    let result = run(&mut |event| {
        let phase = match event {
            SyncEvent::Checking => "checking dependencies",
            SyncEvent::CheckingLockfile => "checking lockfile",
            SyncEvent::LockfileChecked { changed } => {
                active_detail = Some(
                    if changed {
                        "updated lockfile"
                    } else {
                        "unchanged"
                    }
                    .into(),
                );
                return;
            }
            SyncEvent::Installing => "installing dependencies",
            SyncEvent::Output { line, .. } => {
                progress.worker_output(0, line);
                return;
            }
        };
        if let Some(previous) = active_phase.replace(phase) {
            progress.event(previous, 1, 1, active_detail.take());
        }
        progress.event(phase, 0, 1, None);
        progress.worker_event(Some(0), phase, 0, 0, Some("Bundler".into()));
    });
    match result {
        Ok(result) => {
            progress.worker_finished(0);
            if let Some(phase) = active_phase {
                progress.event(phase, 1, 1, active_detail);
            }
            let summary = match result {
                SyncResult::AlreadySynced => "Everything is already in order",
                SyncResult::Synchronized => "Your bundle is ready",
            };
            match completion {
                Completion::Tree => reporter.finish_with_summary(summary),
                Completion::Compact => reporter.finish_compact(summary),
            }
            Ok(())
        }
        Err(error) => {
            progress.worker_failed(0, error.to_string());
            reporter.finish();
            Err(ButlerError::General(error.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Clone, Default)]
    struct Buffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for Buffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn reporter_maps_bundler_phases_and_success() {
        for result in [SyncResult::AlreadySynced, SyncResult::Synchronized] {
            let buffer = Buffer::default();
            let reporter = Reporter::with_writer("sync", buffer.clone());
            report_sync(reporter, Completion::Tree, |handler| {
                handler(SyncEvent::Checking);
                handler(SyncEvent::CheckingLockfile);
                handler(SyncEvent::Installing);
                handler(SyncEvent::Output {
                    line: "compiler output",
                    stderr: true,
                });
                Ok(result.clone())
            })
            .unwrap();
            let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
            for phase in [
                "checking dependencies",
                "checking lockfile",
                "installing dependencies",
                "compiler output",
            ] {
                assert!(text.contains(phase), "{text}");
            }
            assert!(text.contains("worker 0"));
            assert!(text.contains("│        compiler output"));
            assert!(!text.contains("| compiler output"));
            assert!(text.contains(match result {
                SyncResult::AlreadySynced => "Everything is already in order",
                SyncResult::Synchronized => "Your bundle is ready",
            }));
        }
    }

    #[test]
    fn reporter_retains_bundler_failure() {
        let buffer = Buffer::default();
        let reporter = Reporter::with_writer("sync", buffer.clone());
        let result = report_sync(reporter, Completion::Tree, |handler| {
            handler(SyncEvent::Installing);
            Err(std::io::Error::other("compiler failed"))
        });
        assert!(result.unwrap_err().to_string().contains("compiler failed"));
        let text = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        assert!(text.contains("compiler failed"));
        assert!(text.contains("└─ × failed"));
        assert!(!text.contains("Your bundle is ready"));
    }
}
