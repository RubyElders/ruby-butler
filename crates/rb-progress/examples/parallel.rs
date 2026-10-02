use rb_progress::Reporter;
use std::{thread, time::Duration};

fn main() {
    let reporter = Reporter::start("Preparing the dining room").unwrap();
    let progress = reporter.handle();
    progress.event("preparing", 0, 3, None);

    thread::scope(|scope| {
        for (worker, job) in ["polishing silver", "folding napkins", "arranging flowers"]
            .into_iter()
            .enumerate()
        {
            let progress = progress.clone();
            scope.spawn(move || {
                progress.worker_started(worker, job);
                for done in 1..=3 {
                    thread::sleep(Duration::from_millis(300));
                    progress.worker_event(
                        Some(worker),
                        job,
                        done,
                        3,
                        Some(format!("{done}/3 pieces")),
                    );
                }
                progress.worker_finished(worker);
            });
        }
    });

    progress.event("preparing", 3, 3, None);
    reporter.finish_with_summary("The dining room is ready, sir");
}
