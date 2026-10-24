use super::sync_report::Completion;
use crate::packages::PackageRuntime;
use rb_core::butler::ButlerError;

pub(crate) fn sync_command(
    runtime: PackageRuntime,
    backend: &crate::packages::PackageBackend,
) -> Result<(), ButlerError> {
    super::sync_report::report_packages(Completion::Tree, |events| {
        backend.prepare(&runtime, events)
    })?;
    Ok(())
}
