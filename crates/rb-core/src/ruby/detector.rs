use regex::Regex;
use semver::Version;
use std::{fs, path::{Path, PathBuf}};
use log::{debug, info};

use super::{RubyRuntime, RubyType, RubyDiscoveryError};

pub struct RubyRuntimeDetector;

impl RubyRuntimeDetector {
    pub fn discover(root_dir: &Path) -> Result<Vec<RubyRuntime>, RubyDiscoveryError> {
        debug!("Starting Ruby discovery in directory: {}", root_dir.display());
        
        // Check if the directory exists first
        if !root_dir.exists() {
            debug!("Ruby discovery directory does not exist: {}", root_dir.display());
            return Err(RubyDiscoveryError::DirectoryNotFound(root_dir.to_path_buf()));
        }
        
        let mut out = Vec::new();
        let re = Regex::new(r"^ruby-(\d+)\.(\d+)\.(\d+)$").expect("static regex");

        let entries = fs::read_dir(root_dir)
            .map_err(|e| RubyDiscoveryError::IoError(format!("Failed to read directory {}: {}", root_dir.display(), e)))?;

        for entry in entries {
            let entry = entry.map_err(|e| RubyDiscoveryError::IoError(e.to_string()))?;
            let file_type = entry.file_type().map_err(|e| RubyDiscoveryError::IoError(e.to_string()))?;
            
            if !file_type.is_dir() { 
                debug!("Skipping non-directory entry: {}", entry.path().display());
                continue; 
            }
            let name = entry.file_name().to_string_lossy().to_string();
            debug!("Examining directory: {}", name);

            if let Some(c) = re.captures(&name) {
                let v = format!("{}.{}.{}", &c[1], &c[2], &c[3]);
                debug!("Found potential Ruby directory matching pattern: {} -> version {}", name, v);
                
                if let Ok(version) = Version::parse(&v) {
                    let root: PathBuf = entry.path();
                    debug!("Successfully parsed version {} for Ruby at: {}", version, root.display());
                    out.push(RubyRuntime { kind: RubyType::CRuby, version, root });
                } else {
                    debug!("Failed to parse version from directory name: {}", name);
                }
            } else {
                debug!("Directory name {} does not match Ruby directory pattern", name);
            }
        }

        out.sort_by(|a, b| b.version.cmp(&a.version)); // latest first
        info!("Discovered {} Ruby installations in {}", out.len(), root_dir.display());
        
        for ruby in &out {
            debug!("Found Ruby: {} {} at {}", ruby.kind.as_str(), ruby.version, ruby.root.display());
        }
        
        Ok(out)
    }

    pub fn latest(list: &[RubyRuntime]) -> Option<RubyRuntime> {
        let result = list.iter().max_by(|a, b| a.version.cmp(&b.version)).cloned();
        
        if let Some(ref latest) = result {
            debug!("Latest Ruby determined: {} {} at {}", latest.kind.as_str(), latest.version, latest.root.display());
        } else {
            debug!("No Ruby installations found, cannot determine latest");
        }
        
        result
    }
}

