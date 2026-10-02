use super::sync_report::{Completion, report_sync, sync_reporter};
use log::debug;
use rb_core::butler::{ButlerError, ButlerRuntime};

pub fn sync_command(butler_runtime: ButlerRuntime) -> Result<(), ButlerError> {
    debug!("Starting sync command");

    let bundler_runtime = match butler_runtime.bundler_runtime() {
        Some(bundler) => bundler,
        None => {
            return Err(ButlerError::General(
                "Bundler environment not detected.\n\nNo Gemfile found in the current directory or its ancestors.\nThe sync command requires a bundler-managed project to operate.\n\nTo create a new bundler project, create a Gemfile with: echo 'source \"https://rubygems.org\"' > Gemfile".to_string()
            ));
        }
    };

    report_sync(sync_reporter(), Completion::Tree, |handler| {
        bundler_runtime.synchronize_with_events(&butler_runtime, handler)
    })
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
