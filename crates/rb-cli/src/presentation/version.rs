use crate::commands::version::VersionReport;

pub(crate) fn text(report: &VersionReport) -> String {
    let version = report.version.as_str();
    let git_hash = report.git_hash.as_str();
    let profile = report.profile.as_str();

    let mut parts = vec![format!("Ruby Butler v{}", version)];

    if let Some(tag) = report.tag.as_deref() {
        if !tag.is_empty() && tag != format!("v{}", version) {
            parts.push(format!("({})", tag));
        }
    } else if git_hash != "unknown" {
        parts.push(format!("({})", git_hash));
    }

    if profile == "debug" {
        parts.push("[debug build]".to_string());
    }

    if report.modified {
        parts.push("[modified]".to_string());
    }

    parts.push(
        "\n\nA sophisticated Ruby environment manager with the refined precision".to_string(),
    );
    parts.push("of a proper gentleman's gentleman.\n".to_string());
    parts.push("At your distinguished service, RubyElders.com".to_string());

    parts.join(" ")
}
