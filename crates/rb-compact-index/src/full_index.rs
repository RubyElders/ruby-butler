use crate::{
    Client, GemInfo,
    cache::{atomic_write, checksum, limited_read, lock},
    constants::{
        ARCHIVE_CACHE_DIRECTORY, ARCHIVE_PATH_PREFIX, FULL_INFO_CACHE_SUFFIX, GEM_METADATA_PATH,
        MAX_COMPRESSED_INDEX_BYTES, MAX_DECOMPRESSED_INDEX_BYTES, MAX_FALLBACK_ARCHIVE_BYTES,
        MAX_GEM_METADATA_BYTES, PRERELEASE_INDEX_PATH, RELEASE_INDEX_PATH,
    },
    validation::validate_name,
};
use alox_48::Value;
use anyhow::{Context, Result, bail, ensure};
use flate2::read::GzDecoder;
use serde_yaml_ng::Value as Yaml;
use std::{collections::BTreeMap, fs, path::Path};

fn rows(bytes: &[u8]) -> Result<Vec<(String, String)>> {
    let data = limited_read(GzDecoder::new(bytes), MAX_DECOMPRESSED_INDEX_BYTES)?;
    let value: Value = alox_48::from_bytes(&data)?;
    let entries = value.as_array().context("Invalid RubyGems full index")?;
    let mut result = Vec::new();
    for entry in entries {
        let entry = entry.as_array().context("Invalid full-index entry")?;
        ensure!(entry.len() == 3, "Invalid full-index entry length");
        let platform: String = alox_48::from_value(&entry[2])?;
        let name: String = alox_48::from_value(&entry[0])?;
        let Value::UserMarshal { class, value } = &entry[1] else {
            bail!("Invalid full-index version");
        };
        ensure!(class == "Gem::Version", "Invalid full-index version class");
        let version: Vec<String> = alox_48::from_value(value)?;
        ensure!(version.len() == 1, "Invalid full-index version fields");
        validate_name(&name)?;
        validate_name(&version[0])?;
        validate_name(&platform)?;
        result.push((
            name,
            if platform == "ruby" {
                version[0].clone()
            } else {
                format!("{}-{platform}", version[0])
            },
        ));
    }
    Ok(result)
}

fn untag(mut node: &Yaml) -> &Yaml {
    while let Yaml::Tagged(inner) = node {
        node = &inner.value;
    }
    node
}
fn field<'a>(node: &'a Yaml, key: &str) -> &'a Yaml {
    untag(node).get(key).map(untag).unwrap_or(&Yaml::Null)
}
fn requirement(node: &Yaml) -> Result<String> {
    let pairs = field(node, "requirements")
        .as_sequence()
        .context("Missing gem requirement")?;
    pairs
        .iter()
        .map(|pair| {
            let pair = pair.as_sequence().context("Invalid gem requirement")?;
            ensure!(pair.len() == 2, "Invalid gem requirement length");
            Ok(format!(
                "{} {}",
                pair[0].as_str().context("Missing requirement operator")?,
                field(&pair[1], "version")
                    .as_str()
                    .context("Missing requirement version")?
            ))
        })
        .collect::<Result<Vec<_>>>()
        .map(|parts| parts.join(", "))
}
fn metadata(bytes: &[u8], name: &str, version: &str) -> Result<GemInfo> {
    let mut metadata = None;
    for entry in tar::Archive::new(bytes).entries()? {
        let entry = entry?;
        if entry.path()?.as_ref() == Path::new(GEM_METADATA_PATH) {
            ensure!(metadata.is_none(), "Duplicate gem metadata");
            metadata = Some(limited_read(GzDecoder::new(entry), MAX_GEM_METADATA_BYTES)?);
        }
    }
    let text = String::from_utf8(metadata.context("Missing gem metadata")?)?;
    let document: Yaml = serde_yaml_ng::from_str(&text)?;
    let spec = untag(&document);
    ensure!(
        field(spec, "name").as_str() == Some(name),
        "Archive name differs from full index"
    );
    ensure!(
        field(field(spec, "version"), "version").as_str()
            == Some(version.split_once('-').map_or(version, |(v, _)| v)),
        "Archive version differs from full index"
    );
    let platform = field(spec, "platform");
    let platform = if platform.is_null() {
        "ruby".to_string()
    } else if let Some(value) = platform.as_str() {
        value.to_string()
    } else {
        ["cpu", "os", "version"]
            .into_iter()
            .filter_map(|key| field(platform, key).as_str())
            .collect::<Vec<_>>()
            .join("-")
    };
    ensure!(
        platform == version.split_once('-').map_or("ruby", |(_, p)| p),
        "Archive platform differs from full index"
    );
    let mut dependencies = BTreeMap::<String, String>::new();
    if let Some(items) = field(spec, "dependencies").as_sequence() {
        for item in items {
            if field(item, "type")
                .as_str()
                .is_some_and(|kind| kind.trim_start_matches(':') != "runtime")
            {
                continue;
            }
            let name = field(item, "name")
                .as_str()
                .context("Missing dependency name")?;
            validate_name(name)?;
            let constraint = requirement(field(item, "requirement"))?;
            dependencies
                .entry(name.into())
                .and_modify(|value| {
                    value.push_str(", ");
                    value.push_str(&constraint);
                })
                .or_insert(constraint);
        }
    }
    Ok(GemInfo {
        version: version.into(),
        dependencies,
        metadata: BTreeMap::from([
            ("checksum".into(), checksum(bytes)),
            (
                "ruby".into(),
                requirement(field(spec, "required_ruby_version"))?,
            ),
        ]),
    })
}

impl Client {
    pub(super) fn full_info(&self, name: &str, directory: &Path) -> Result<Vec<GemInfo>> {
        let mut versions = Vec::new();
        for path in [RELEASE_INDEX_PATH, PRERELEASE_INDEX_PATH] {
            let response = self.client.get(format!("{}{path}", self.source)).send()?;
            if path == PRERELEASE_INDEX_PATH && response.status() == reqwest::StatusCode::NOT_FOUND
            {
                continue;
            }
            versions.extend(rows(&limited_read(
                response.error_for_status()?,
                MAX_COMPRESSED_INDEX_BYTES,
            )?)?);
        }
        versions.sort();
        versions.dedup();
        let mut gems = Vec::new();
        for (_, version) in versions.into_iter().filter(|(gem, _)| gem == name) {
            let bytes = limited_read(
                self.client
                    .get(format!(
                        "{}{ARCHIVE_PATH_PREFIX}{name}-{version}.gem",
                        self.source
                    ))
                    .send()?
                    .error_for_status()?,
                MAX_FALLBACK_ARCHIVE_BYTES,
            )?;
            let info = metadata(&bytes, name, &version)
                .with_context(|| format!("Reading full-index metadata for {name}-{version}"))?;
            let archives = self.cache.join(ARCHIVE_CACHE_DIRECTORY);
            fs::create_dir_all(&archives)?;
            let hash = info.metadata.get("checksum").unwrap();
            let _guard = lock(&archives.join(format!("{hash}.lock")))?;
            atomic_write(&archives.join(format!("{hash}.gem")), &bytes)?;
            gems.push(info);
        }
        atomic_write(
            &directory.join(format!("{name}{FULL_INFO_CACHE_SUFFIX}")),
            &serde_json::to_vec(&gems)?,
        )?;
        Ok(gems)
    }
}
