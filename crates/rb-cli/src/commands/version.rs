use rb_core::butler::ButlerError;

/// Build version information string
pub fn build_version_info() -> String {
    let version = env!("CARGO_PKG_VERSION");
    let git_hash = option_env!("GIT_HASH").unwrap_or("unknown");
    let profile = option_env!("BUILD_PROFILE").unwrap_or("unknown");

    let mut parts = vec![format!("Ruby Butler v{}", version)];

    // Add tag if available, otherwise add git hash
    if let Some(tag) = option_env!("GIT_TAG") {
        if !tag.is_empty() && tag != format!("v{}", version) {
            parts.push(format!("({})", tag));
        }
    } else if git_hash != "unknown" {
        parts.push(format!("({})", git_hash));
    }

    // Add profile if debug
    if profile == "debug" {
        parts.push("[debug build]".to_string());
    }

    // Add dirty flag if present
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
