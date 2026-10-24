use super::{
    BackendStatus, Driver, PackageDetails, PackageEnvironment, PackageEvents, PreparedInstallation,
};
use anyhow::Result;
use rb_core::{bundler::SyncResult, butler::ButlerRuntime};

pub(super) struct BundlerBackend;

impl Driver for BundlerBackend {
    fn inspect(&self, runtime: &ButlerRuntime) -> Result<PackageEnvironment> {
        let details = runtime.bundler_runtime().map(|bundler| PackageDetails {
            root: bundler.root.clone(),
            gemfile: Some(bundler.gemfile_path()),
            configuration: Some(bundler.app_config_dir()),
            installation: bundler.vendor_dir(),
            required_ruby: bundler.ruby_version(),
            configured: bundler.is_configured(),
        });
        Ok(PackageEnvironment {
            runtime: runtime.clone(),
            name: "Bundler",
            details,
        })
    }

    fn check(&self, runtime: &ButlerRuntime) -> Result<BackendStatus> {
        if runtime.bundler_runtime().is_some_and(|bundler| {
            !bundler.check_sync(runtime).unwrap_or_else(|error| {
                log::debug!("Unable to verify bundler synchronization status: {error}");
                true
            })
        }) {
            Ok(BackendStatus::NeedsPreparation)
        } else {
            Ok(BackendStatus::Ready(Box::new(runtime.clone())))
        }
    }

    fn prepare(
        &self,
        runtime: &ButlerRuntime,
        events: PackageEvents<'_>,
    ) -> Result<PreparedInstallation> {
        let bundler = runtime.bundler_runtime().ok_or_else(|| anyhow::anyhow!(
            "Bundler environment not detected.\n\nNo Gemfile found in the current directory or its ancestors.\nThe sync command requires a bundler-managed project to operate.\n\nTo create a new bundler project, create a Gemfile with: echo 'source \"https://rubygems.org\"' > Gemfile"
        ))?;
        let result = bundler.synchronize_with_events(runtime, events.sync)?;
        Ok(PreparedInstallation {
            runtime: runtime.clone(),
            changed: result == SyncResult::Synchronized,
        })
    }
}
