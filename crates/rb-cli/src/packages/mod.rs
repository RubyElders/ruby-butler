mod bundler;
mod native;

use crate::config::PackageBackendKind;
use anyhow::Result;
use rb_core::bundler::SyncEvent;
use rb_core::butler::ButlerRuntime;
use rb_task::TaskEvents;
use std::path::PathBuf;

pub(crate) struct PackageEvents<'a> {
    pub tasks: TaskEvents,
    pub sync: &'a mut dyn FnMut(SyncEvent<'_>),
}

pub(crate) enum PackageStatus {
    Ready(Box<ButlerRuntime>),
    NeedsPreparation,
}

pub(crate) trait PackageBackend {
    fn check(&self, runtime: &ButlerRuntime) -> Result<PackageStatus>;
    fn prepare(
        &self,
        runtime: &ButlerRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedEnvironment>;
}

pub(crate) struct PreparedEnvironment {
    pub runtime: ButlerRuntime,
    pub changed: bool,
}

pub(crate) fn select(
    kind: PackageBackendKind,
    project_file: Option<PathBuf>,
    disabled: bool,
) -> Box<dyn PackageBackend> {
    match kind {
        PackageBackendKind::Bundler => Box::new(bundler::BundlerBackend),
        PackageBackendKind::Native => Box::new(native::NativeBackend::new(project_file, disabled)),
    }
}
