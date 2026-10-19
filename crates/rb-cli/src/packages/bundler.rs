use super::{PackageBackend, PackageEvents, PackageStatus, PreparedEnvironment};
use anyhow::Result;
use rb_core::{bundler::SyncResult, butler::ButlerRuntime};

pub(super) struct BundlerBackend;

impl PackageBackend for BundlerBackend {
    fn check(&self, runtime: &ButlerRuntime) -> Result<PackageStatus> {
        if runtime.bundler_runtime().is_some_and(|bundler| {
            !bundler.check_sync(runtime).unwrap_or_else(|error| {
                log::debug!("Unable to verify bundler synchronization status: {error}");
                true
            })
        }) {
            Ok(PackageStatus::NeedsPreparation)
        } else {
            Ok(PackageStatus::Ready(Box::new(runtime.clone())))
        }
    }

    fn prepare(
        &self,
        runtime: &ButlerRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedEnvironment> {
        let bundler = runtime.bundler_runtime().ok_or_else(|| anyhow::anyhow!(
            "Bundler environment not detected.\n\nNo Gemfile found in the current directory or its ancestors.\nThe sync command requires a bundler-managed project to operate.\n\nTo create a new bundler project, create a Gemfile with: echo 'source \"https://rubygems.org\"' > Gemfile"
        ))?;
        let result = bundler.synchronize_with_events(runtime, events.sync)?;
        Ok(PreparedEnvironment {
            runtime: runtime.clone(),
            changed: result == SyncResult::Synchronized,
        })
    }
}
