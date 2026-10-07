use crate::{
    Dependency, LockedPackage, Lockfile, ParseError, Source, SourceKind,
    constants::{
        DEPENDENCY_INDENT, ENVIRONMENT_INDENT, GIT_REVISION_HEX_LENGTHS, PACKAGE_INDENT,
        SECTION_INDENT, SHA256_HEX_LENGTH,
    },
};
use rb_gem_types::Requirement;
use std::collections::{BTreeMap, BTreeSet};

fn error(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

fn name(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_.-".contains(&c))
}

fn declaration(value: &str, line: usize) -> Result<(&str, Option<&str>), ParseError> {
    let (gem, version) = match value.split_once(" (") {
        Some((gem, version)) => (
            gem,
            Some(
                version
                    .strip_suffix(')')
                    .ok_or_else(|| error(line, "Unclosed declaration"))?,
            ),
        ),
        None => (value, None),
    };
    if !name(gem) {
        return Err(error(line, "Invalid gem name"));
    }
    Ok((gem, version))
}

fn dependency(value: &str, line: usize) -> Result<(String, Requirement), ParseError> {
    let (gem, version) = declaration(value, line)?;
    let requirement = version
        .unwrap_or(">= 0")
        .parse()
        .map_err(|e| error(line, format!("{e}")))?;
    Ok((gem.into(), requirement))
}

fn package(value: &str, line: usize) -> Result<LockedPackage, ParseError> {
    let (gem, archive) = declaration(value, line)?;
    let archive = archive.ok_or_else(|| error(line, "Missing package version"))?;
    let (version, platform) = archive.split_once('-').unwrap_or((archive, "ruby"));
    if !name(platform) {
        return Err(error(line, "Invalid platform"));
    }
    Ok(LockedPackage {
        name: gem.into(),
        version: version.parse().map_err(|e| error(line, format!("{e}")))?,
        platform: platform.into(),
        dependencies: BTreeMap::new(),
    })
}

pub(crate) fn parse(text: &str) -> Result<Lockfile, ParseError> {
    let mut lock = Lockfile::default();
    let mut section = "";
    let mut sections = BTreeSet::new();
    let mut specs = false;
    for (offset, raw) in text.lines().enumerate() {
        let line = offset + 1;
        if raw.trim().is_empty() {
            continue;
        }
        if raw.contains(['\t', '\0', '\r']) {
            return Err(error(line, "Invalid control character"));
        }
        let indent = raw.len() - raw.trim_start_matches(' ').len();
        let value = &raw[indent..];
        if indent == 0 {
            if matches!(section, "GEM" | "GIT" | "PATH") && !specs {
                return Err(error(line, "Source is missing specs"));
            }
            section = value;
            specs = false;
            let kind = match section {
                "GEM" => Some(SourceKind::Registry),
                "GIT" => Some(SourceKind::Git),
                "PATH" => Some(SourceKind::Path),
                "PLATFORMS" | "DEPENDENCIES" | "CHECKSUMS" | "RUBY VERSION" | "BUNDLED WITH" => {
                    None
                }
                _ => return Err(error(line, format!("Unsupported section: {section}"))),
            };
            if let Some(kind) = kind {
                lock.sources.push(Source {
                    kind,
                    remotes: Vec::new(),
                    options: BTreeMap::new(),
                    packages: Vec::new(),
                });
            } else if !sections.insert(section) {
                return Err(error(line, "Duplicate section"));
            }
            if section == "CHECKSUMS" {
                lock.checksums = Some(BTreeMap::new());
            }
            continue;
        }
        match section {
            "GEM" | "GIT" | "PATH" => {
                let source = lock.sources.last_mut().unwrap();
                match indent {
                    SECTION_INDENT if value == "specs:" && !specs => {
                        validate_source(source, line)?;
                        specs = true;
                    }
                    SECTION_INDENT if !specs => {
                        let (key, value) = value
                            .split_once(": ")
                            .ok_or_else(|| error(line, "Invalid source option"))?;
                        if value.is_empty() {
                            return Err(error(line, "Empty source option"));
                        }
                        if key == "remote" {
                            source.remotes.push(value.into());
                        } else {
                            let allowed = match source.kind {
                                SourceKind::Git => matches!(
                                    key,
                                    "revision" | "branch" | "tag" | "ref" | "submodules" | "glob"
                                ),
                                SourceKind::Path => matches!(key, "glob"),
                                SourceKind::Registry => false,
                            };
                            if !allowed || source.options.insert(key.into(), value.into()).is_some()
                            {
                                return Err(error(line, "Unsupported or duplicate source option"));
                            }
                        }
                    }
                    PACKAGE_INDENT if specs => source.packages.push(package(value, line)?),
                    DEPENDENCY_INDENT if specs => {
                        let (gem, constraint) = dependency(value, line)?;
                        let package = source
                            .packages
                            .last_mut()
                            .ok_or_else(|| error(line, "Dependency without a package"))?;
                        package
                            .dependencies
                            .entry(gem)
                            .or_default()
                            .push(constraint);
                    }
                    _ => return Err(error(line, "Invalid source indentation or content")),
                }
            }
            "DEPENDENCIES" if indent == SECTION_INDENT => {
                let pinned = value.ends_with('!');
                let (gem, requirement) =
                    dependency(value.strip_suffix('!').unwrap_or(value), line)?;
                if lock
                    .dependencies
                    .insert(
                        gem,
                        Dependency {
                            requirement,
                            pinned,
                        },
                    )
                    .is_some()
                {
                    return Err(error(line, "Duplicate root dependency"));
                }
            }
            "PLATFORMS" if indent == SECTION_INDENT && name(value) => {
                if lock.platforms.iter().any(|p| p == value) {
                    return Err(error(line, "Duplicate platform"));
                }
                lock.platforms.push(value.into());
            }
            "CHECKSUMS" if indent == SECTION_INDENT => {
                let (identity, hash) = value
                    .split_once(" sha256=")
                    .map_or((value, None), |(id, hash)| (id, Some(hash)));
                let parsed = package(identity, line)?;
                if hash.is_some_and(|hash| {
                    hash.len() != SHA256_HEX_LENGTH || !hash.bytes().all(|c| c.is_ascii_hexdigit())
                }) {
                    return Err(error(line, "Invalid SHA-256 checksum"));
                }
                if lock
                    .checksums
                    .as_mut()
                    .unwrap()
                    .insert(parsed.identity(), hash.map(String::from))
                    .is_some()
                {
                    return Err(error(line, "Duplicate checksum"));
                }
            }
            "RUBY VERSION" if matches!(indent, SECTION_INDENT | ENVIRONMENT_INDENT) => {
                if !value.starts_with("ruby ")
                    || value.trim() != value
                    || lock.ruby_version.replace(value.into()).is_some()
                {
                    return Err(error(line, "Invalid or duplicate Ruby version"));
                }
                let version = value[5..].split(['p', ' ']).next().unwrap();
                version
                    .parse::<rb_gem_types::Version>()
                    .map_err(|e| error(line, format!("{e}")))?;
            }
            "BUNDLED WITH" if matches!(indent, SECTION_INDENT | ENVIRONMENT_INDENT) => {
                let version = value.parse().map_err(|e| error(line, format!("{e}")))?;
                if lock.bundled_with.replace(version).is_some() {
                    return Err(error(line, "Duplicate Bundler version"));
                }
            }
            _ => return Err(error(line, "Unexpected content or indentation")),
        }
    }
    if matches!(section, "GEM" | "GIT" | "PATH") && !specs {
        return Err(error(0, "Source is missing specs"));
    }
    if lock.sources.is_empty()
        || !sections.contains("PLATFORMS")
        || !sections.contains("DEPENDENCIES")
        || lock.platforms.is_empty()
    {
        return Err(error(
            0,
            "Missing sources, platforms or dependencies section",
        ));
    }
    if (sections.contains("RUBY VERSION") && lock.ruby_version.is_none())
        || (sections.contains("BUNDLED WITH") && lock.bundled_with.is_none())
    {
        return Err(error(0, "Empty environment version section"));
    }
    let mut identities = BTreeSet::new();
    for source in &lock.sources {
        validate_source(source, 0)?;
        for package in &source.packages {
            if !identities.insert(package.identity()) {
                return Err(error(0, "Duplicate package identity"));
            }
        }
    }
    let implicit_bundler = lock
        .bundled_with
        .as_ref()
        .map(|version| format!("bundler ({version})"));
    if lock.checksums.as_ref().is_some_and(|checksums| {
        checksums
            .keys()
            .any(|id| !identities.contains(id) && Some(id) != implicit_bundler.as_ref())
    }) {
        return Err(error(0, "Checksum without a package"));
    }
    Ok(lock)
}

fn validate_source(source: &Source, line: usize) -> Result<(), ParseError> {
    if source.kind != SourceKind::Registry && source.remotes.len() != 1 {
        return Err(error(line, "Missing or ambiguous source remote"));
    }
    if source.kind == SourceKind::Git {
        if !source.options.get("revision").is_some_and(|rev| {
            GIT_REVISION_HEX_LENGTHS.contains(&rev.len())
                && rev.bytes().all(|c| c.is_ascii_hexdigit())
        }) {
            return Err(error(line, "Invalid or missing Git revision"));
        }
        if source
            .options
            .get("submodules")
            .is_some_and(|v| v != "true" && v != "false")
        {
            return Err(error(line, "Invalid submodules value"));
        }
    }
    Ok(())
}
