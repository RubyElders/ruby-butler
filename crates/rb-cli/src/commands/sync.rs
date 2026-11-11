use crate::packages::PackageRuntime;
use rb_core::butler::ButlerError;

pub(crate) fn sync_command(
    runtime: PackageRuntime,
    backend: &crate::packages::PackageBackend,
    report: &mut crate::reporting::PreparationReport,
    output: &crate::output::Output,
) -> Result<(), ButlerError> {
    let result =
        super::sync_report::report_packages(report, |events| backend.prepare(&runtime, events))?;
    let report = SyncReport {
        changed: result.changed,
    };
    output.emit("sync", &report, crate::output::document::Document::default)
}

#[derive(serde::Serialize)]
struct SyncReport {
    changed: bool,
}
