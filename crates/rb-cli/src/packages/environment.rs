use rb_core::butler::ButlerRuntime;
use std::path::PathBuf;

pub(crate) struct PackageDetails {
    pub root: PathBuf,
    pub gemfile: Option<PathBuf>,
    pub configuration: Option<PathBuf>,
    pub installation: PathBuf,
    pub required_ruby: Option<semver::Version>,
    pub configured: bool,
}

pub(crate) struct PackageEnvironment {
    pub(super) runtime: ButlerRuntime,
    pub name: &'static str,
    pub details: Option<PackageDetails>,
}

impl PackageEnvironment {
    pub fn current_dir(&self) -> &std::path::Path {
        self.runtime.current_dir()
    }
    pub fn ruby(&self) -> Result<&rb_core::ruby::RubyRuntime, rb_core::butler::ButlerError> {
        self.runtime.selected_ruby()
    }
    pub fn gem_home(&self) -> Option<PathBuf> {
        self.runtime.gem_home()
    }
    pub fn gem_dirs(&self) -> Vec<PathBuf> {
        self.runtime.gem_dirs()
    }
    pub fn bin_dirs(&self) -> Vec<PathBuf> {
        self.runtime.bin_dirs()
    }
}
