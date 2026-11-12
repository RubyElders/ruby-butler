mod bundler;
mod cache;
mod environment;
mod native;
mod runtime;

pub(crate) use cache::PackageCache;
pub(crate) use environment::{PackageDetails, PackageEnvironment};
pub(crate) use runtime::{PackageRuntime, ReadyRuntime};

use crate::config::PackageBackendKind;
use anyhow::Result;
use rb_core::bundler::SyncEvent;
use rb_core::butler::ButlerRuntime;
use rb_task::TaskEvents;

pub(crate) struct PackageEvents<'a> {
    pub tasks: TaskEvents,
    pub sync: &'a mut dyn FnMut(SyncEvent<'_>),
}

enum BackendStatus {
    Ready(Box<ButlerRuntime>),
    NeedsPreparation,
}

pub(crate) enum PackageStatus {
    Ready(Box<ReadyRuntime>),
    NeedsPreparation,
}

trait Driver {
    fn location(&self) -> Option<crate::invocation::DependencyLocation>;
    fn inspect(&self, runtime: &ButlerRuntime) -> Result<PackageEnvironment>;
    fn cache_inputs(&self, runtime: &ButlerRuntime) -> Result<Option<PackageCache>>;
    fn check(&self, runtime: &ButlerRuntime) -> Result<BackendStatus>;
    fn prepare(
        &self,
        runtime: &ButlerRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedInstallation>;
}

pub(crate) struct PackageBackend {
    driver: Box<dyn Driver>,
}

impl PackageBackend {
    pub fn from_context(context: &crate::runtime_helpers::CommandContext) -> Self {
        let driver: Box<dyn Driver> = match context.config().package_backend.get() {
            PackageBackendKind::Bundler => Box::new(bundler::BundlerBackend::new(context)),
            PackageBackendKind::Native => Box::new(native::NativeBackend::new(context)),
        };
        Self { driver }
    }

    pub fn location(&self) -> Option<crate::invocation::DependencyLocation> {
        self.driver.location()
    }

    pub fn inspect(&self, runtime: &PackageRuntime) -> Result<PackageEnvironment> {
        self.driver.inspect(&runtime.0)
    }

    pub fn cache_inputs(&self, runtime: &PackageRuntime) -> Result<Option<PackageCache>> {
        self.driver.cache_inputs(&runtime.0)
    }

    pub fn check(&self, runtime: &PackageRuntime) -> Result<PackageStatus> {
        Ok(match self.driver.check(&runtime.0)? {
            BackendStatus::Ready(runtime) => PackageStatus::Ready(Box::new(ReadyRuntime(*runtime))),
            BackendStatus::NeedsPreparation => PackageStatus::NeedsPreparation,
        })
    }

    pub fn prepare(
        &self,
        runtime: &PackageRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedEnvironment> {
        let prepared = self.driver.prepare(&runtime.0, events)?;
        Ok(PreparedEnvironment {
            runtime: ReadyRuntime(prepared.runtime),
            changed: prepared.changed,
        })
    }
}

struct PreparedInstallation {
    runtime: ButlerRuntime,
    changed: bool,
}

pub(crate) struct PreparedEnvironment {
    pub runtime: ReadyRuntime,
    pub changed: bool,
}
