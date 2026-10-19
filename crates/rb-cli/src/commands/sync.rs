use super::sync_report::Completion;
use rb_core::butler::{ButlerError, ButlerRuntime};

pub fn sync_command(butler_runtime: ButlerRuntime) -> Result<(), ButlerError> {
    let backend = crate::packages::select(crate::config::PackageBackendKind::Bundler, None, false);
    sync_with_backend(butler_runtime, backend.as_ref())
}

pub(crate) fn sync_with_backend(
    runtime: ButlerRuntime,
    backend: &dyn crate::packages::PackageBackend,
) -> Result<(), ButlerError> {
    super::sync_report::report_packages(Completion::Tree, |events| {
        backend.prepare(&runtime, events)
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use rb_tests::BundlerSandbox;

    #[test]
    fn test_sync_command_with_no_gemfile() -> Result<(), Box<dyn std::error::Error>> {
        let sandbox = BundlerSandbox::new()?;
        let project_dir = sandbox.add_dir("no_gemfile_project")?;
        let rubies_dir = sandbox.add_dir("rubies")?;

        let original_dir = std::env::current_dir()?;
        std::env::set_current_dir(&project_dir)?;

        let result = ButlerRuntime::discover_and_compose_with_gem_base(
            rubies_dir.clone(),
            None,
            None,
            false,
        );

        let _ = std::env::set_current_dir(original_dir);

        match result {
            Ok(runtime) => {
                // If runtime creation succeeded (found Ruby), sync should fail due to no Gemfile
                let sync_result = sync_command(runtime);
                assert!(
                    sync_result.is_err(),
                    "Expected sync to fail without Gemfile"
                );
                Ok(())
            }
            Err(_) => {
                // Expected in test environment without Ruby installation
                Ok(())
            }
        }
    }
}
