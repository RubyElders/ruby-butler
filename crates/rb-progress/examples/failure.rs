use rb_progress::Reporter;
use std::{thread, time::Duration};

fn inspect_teacup() -> Result<(), String> {
    thread::sleep(Duration::from_millis(800));
    Err("Replace the teacup".into())
}

fn main() {
    let reporter = Reporter::start("Inspecting the china").unwrap();
    let progress = reporter.handle();
    progress.event("inspecting", 0, 1, None);
    match inspect_teacup() {
        Ok(()) => progress.event("inspecting", 1, 1, None),
        Err(error) => progress.fail(error),
    }
    reporter.finish();
}
