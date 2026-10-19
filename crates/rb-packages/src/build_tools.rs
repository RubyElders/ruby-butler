use rb_gem_archive::GemMetadata;
use std::path::Path;

pub(crate) fn requires_rake(metadata: &GemMetadata) -> bool {
    metadata.extensions.iter().any(|extension| {
        matches!(
            Path::new(extension)
                .file_name()
                .and_then(|name| name.to_str()),
            Some("Rakefile" | "rakefile" | "mkrf_conf.rb")
        )
    })
}
