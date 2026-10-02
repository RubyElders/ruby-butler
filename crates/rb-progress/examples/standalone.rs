use rb_progress::Reporter;
use std::{thread, time::Duration};

fn main() {
    let reporter = Reporter::start("Polishing the silver").unwrap();
    let handle = reporter.handle();
    for done in 0..=3 {
        handle.event("polishing", done, 3, None);
        thread::sleep(Duration::from_millis(350));
    }
    reporter.finish_with_summary("The silver is ready, sir");
}
