
use semver::Version;
use std::env::consts::EXE_SUFFIX;
use std::path::{Path, PathBuf};
use crate::butler::runtime_provider::RuntimeProvider;
use crate::gems::GemRuntime;

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

    /// Create a GemRuntime based on ~/.gem/ruby/version pattern
    /// 
    /// This creates a GemRuntime pointing to ~/.gem/ruby/<full.version>
    /// which follows the standard user gem installation pattern.
    pub fn infer_gem_runtime(&self) -> Result<GemRuntime, std::io::Error> {
        let home_dir = home::home_dir()
            .ok_or_else(|| std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Could not determine home directory"
            ))?;
        
        let gem_base = home_dir.join(".gem");
        Ok(GemRuntime::for_base_dir(&gem_base, &self.version))
    }
}

impl RuntimeProvider for RubyRuntime {
    fn bin_dir(&self) -> Option<PathBuf> {
        Some(self.bin_dir())
    }
    fn gem_dir(&self) -> Option<PathBuf> {
        Some(self.lib_dir())
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

    #[test]
    fn runtime_provider_returns_bin_and_gem_dir_for_ruby_runtime() {
        let r = rt("3.2.2", "/opt/rubies/ruby-3.2.2");
    let expected_bin = Some(r.root.join("bin"));
    let expected_gem = Some(r.root.join("lib").join("ruby").join("gems").join("3.2.0"));
    assert_eq!(<RubyRuntime as RuntimeProvider>::bin_dir(&r), expected_bin);
    assert_eq!(<RubyRuntime as RuntimeProvider>::gem_dir(&r), expected_gem);
    }

    #[test]
    fn infer_gem_runtime_creates_proper_gem_runtime() {
        // Use a simple test since home crate handles cross-platform concerns
        let r = rt("3.4.5", "/opt/rubies/ruby-3.4.5");
        let gem_runtime = r.infer_gem_runtime().expect("Should create GemRuntime");
        
        // Check that the gem_home follows /.gem/ruby/3.4.5 pattern (full version)
        assert!(gem_runtime.gem_home.ends_with(Path::new(".gem").join("ruby").join("3.4.5")));
        assert!(gem_runtime.gem_bin.ends_with(Path::new(".gem").join("ruby").join("3.4.5").join("bin")));
        
        // Verify the version formatting uses full version
        let version_part = gem_runtime.gem_home.file_name().unwrap();
        assert_eq!(version_part, "3.4.5");
    }
}

pub mod detector;
pub use detector::RubyRuntimeDetector;
