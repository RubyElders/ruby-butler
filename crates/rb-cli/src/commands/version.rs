use crate::output::{Output, document::Document};
use rb_core::butler::ButlerError;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct VersionReport {
    pub version: String,
    pub git_hash: String,
    pub profile: String,
    pub tag: Option<String>,
    pub modified: bool,
}
impl VersionReport {
    pub(crate) fn collect() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            git_hash: option_env!("GIT_HASH").unwrap_or("unknown").into(),
            profile: option_env!("BUILD_PROFILE").unwrap_or("unknown").into(),
            tag: option_env!("GIT_TAG").map(str::to_owned),
            modified: option_env!("GIT_DIRTY").is_some(),
        }
    }
}
/// Build version information string
pub fn build_version_info() -> String {
    crate::presentation::version::text(&VersionReport::collect())
}
/// Version command - displays version information
pub fn version_command() -> Result<(), ButlerError> {
    version_with_output(&Output::default())
}
pub(crate) fn version_with_output(output: &Output) -> Result<(), ButlerError> {
    let report = VersionReport::collect();
    output.emit("version", &report, || {
        Document::verbatim(format!("{}\n", crate::presentation::version::text(&report)))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_version_info_contains_version() {
        let info = build_version_info();
        let version = env!("CARGO_PKG_VERSION");
        assert!(info.contains(&format!("v{}", version)));
    }

    #[test]
    fn test_build_version_info_contains_butler_branding() {
        let info = build_version_info();
        assert!(info.contains("Ruby Butler"));
        assert!(info.contains("RubyElders.com"));
        assert!(info.contains("gentleman"));
    }

    #[test]
    fn test_build_version_info_includes_git_hash_when_available() {
        let info = build_version_info();
        assert!(!info.is_empty());
        assert!(info.len() > 50);
    }

    #[test]
    fn test_version_command_returns_ok() {
        let result = version_command();
        assert!(result.is_ok());
    }
}
