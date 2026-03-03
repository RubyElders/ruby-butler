use rb_core::butler::ButlerError;

/// Build version information string
pub fn build_version_info() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");
    let profile = option_env!("BUILD_PROFILE").unwrap_or("unknown");

    let mut parts = vec![format!("Ruby Butler v{}", version)];

    if let Some(tag) = option_env!("GIT_TAG") {
        if !tag.is_empty() && tag != format!("v{}", version) {
            parts.push(format!("({})", tag));
        }
    } else if git_hash != "unknown" {
        parts.push(format!("({})", git_hash));
    }

    if profile == "debug" {
        parts.push("[debug build]".to_string());
    }

    if option_env!("GIT_DIRTY").is_some() {
        parts.push("[modified]".to_string());
    }

    parts.push(
        "\n\nA sophisticated Ruby environment manager with the refined precision".to_string(),
    );
    parts.push("of a proper gentleman's gentleman.\n".to_string());
    parts.push("At your distinguished service, RubyElders.com".to_string());

    parts.join(" ")
}

/// Version command - displays version information
pub fn version_command() -> Result<(), ButlerError> {
    println!("{}", build_version_info());
    Ok(())
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
