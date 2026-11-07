use crate::packages::PackageRuntime;
use rb_core::butler::ButlerError;

pub(crate) fn sync_command(
    runtime: PackageRuntime,
    backend: &crate::packages::PackageBackend,
    report: &mut crate::reporting::PreparationReport,
) -> Result<(), ButlerError> {
    super::sync_report::report_packages(report, |events| backend.prepare(&runtime, events))?;
    Ok(())
}
