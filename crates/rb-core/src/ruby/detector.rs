use regex::Regex;
use semver::Version;
use std::{fs, path::{Path, PathBuf}};

use super::{RubyRuntime, RubyType};

pub struct RubyRuntimeDetector;

impl RubyRuntimeDetector {
    pub fn discover(root_dir: &Path) -> std::io::Result<Vec<RubyRuntime>> {
        let mut out = Vec::new();
        let re = Regex::new(r"^ruby-(\d+)\.(\d+)\.(\d+)$").expect("static regex");

        for entry in fs::read_dir(root_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() { continue; }
            let name = entry.file_name().to_string_lossy().to_string();

            if let Some(c) = re.captures(&name) {
                let v = format!("{}.{}.{}", &c[1], &c[2], &c[3]);
                if let Ok(version) = Version::parse(&v) {
                    let root: PathBuf = entry.path();
                    out.push(RubyRuntime { kind: RubyType::CRuby, version, root });
                }
            }
        }

        out.sort_by(|a, b| b.version.cmp(&a.version)); // latest first
        Ok(out)
    }

    pub fn latest(list: &[RubyRuntime]) -> Option<RubyRuntime> {
        list.iter().max_by(|a, b| a.version.cmp(&b.version)).cloned()
    }
}

