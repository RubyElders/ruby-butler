use crate::butler::env_provider::EnvProvider;
impl EnvProvider for RubyRuntime {
    fn env_vars(&self, _current_path: Option<String>) -> Vec<(String, String)> {
        vec![]
    }
    fn extra_path(&self) -> Vec<std::path::PathBuf> {
        vec![self.bin_dir().clone()]
    }
}
use semver::Version;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RubyType {
    /// MRI / CRuby
    CRuby,
}

impl RubyType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RubyType::CRuby => "CRuby",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RubyRuntime {
    pub kind: RubyType,
    pub version: Version,
    pub root: PathBuf,
}

impl RubyRuntime {
    pub fn new(kind: RubyType, version: Version, root: impl AsRef<Path>) -> Self {
        Self {
            kind,
            version,
            root: root.as_ref().to_path_buf(),
        }
    }

    /// Identifier like "CRuby-3.2.1"
    pub fn version_name(&self) -> String {
        format!("{}-{}", self.kind.as_str(), self.version)
    }

    /// `<root>/bin`
    pub fn bin_dir(&self) -> PathBuf {
        self.root.join("bin")
    }

    /// `<root>/bin/ruby{EXE_SUFFIX}`
    pub fn ruby_executable_path(&self) -> PathBuf {
        self.bin_dir().join(format!("ruby{EXE_SUFFIX}"))
    }

    /// `<root>/lib/ruby/gems/<major>.<minor>.0`
    ///
    /// Note: RubyGems uses the ruby ABI dir (major.minor.0).
    /// If you later discover a platform that differs, branch on `self.kind`.
    pub fn lib_dir(&self) -> PathBuf {
        self.root
            .join("lib")
            .join("ruby")
            .join("gems")
            .join(format!("{}.{}.0", self.version.major, self.version.minor))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn rt(ver: &str, root: &str) -> RubyRuntime {
        RubyRuntime::new(RubyType::CRuby, Version::parse(ver).unwrap(), root)
    }

    #[test]
    fn version_name_composes_kind_and_version() {
        let r = rt("3.2.1", "/opt/rubies/ruby-3.2.1");
        assert_eq!(r.version_name(), "CRuby-3.2.1");
    }

    #[test]
    fn bin_dir_is_root_bin() {
        let root = Path::new("opt").join("rubies").join("ruby-3.4.5");
        let r = rt("3.4.5", root.to_str().unwrap());
        let expected_tail = Path::new("rubies").join("ruby-3.4.5").join("bin");

        assert!(r.bin_dir().ends_with(&expected_tail));
    }

    #[test]
    fn ruby_executable_matches_platform_suffix() {
        let r = rt("3.3.2", "/opt/rubies/ruby-3.3.2");
        let exe = r.ruby_executable_path();

        // File name should be "ruby" (Unix) or "ruby.exe" (Windows)
        let expected_name = format!("ruby{EXE_SUFFIX}");
        assert_eq!(exe.file_name().unwrap(), expected_name.as_str());

        // Parent dir should be the bin dir
        assert_eq!(exe.parent().unwrap(), r.bin_dir().as_path());
    }

    #[test]
    fn lib_gems_path_uses_major_minor_zero() {
        let r = rt("3.2.4", "/opt/rubies/ruby-3.2.4");
        let p = r.lib_dir();
        let expected_tail = Path::new("lib").join("ruby").join("gems").join("3.2.0");
        assert!(p.ends_with(&expected_tail));
    }
}

pub mod detector;
pub use detector::RubyRuntimeDetector;
