use crate::runtime_helpers::CommandContext;
use rb_core::{
    butler::{ButlerError, ButlerRuntime, Command},
    ruby::RubyRuntime,
};
use std::{
    path::{Path, PathBuf},
    process::ExitStatus,
};

#[derive(Clone)]
pub(crate) struct PackageRuntime(pub(super) ButlerRuntime);

impl PackageRuntime {
    pub fn discover(context: &CommandContext) -> Result<Self, ButlerError> {
        context
            .runtime(*context.config().no_bundler.get())
            .map(Self)
    }

    pub fn rubies_dir(&self) -> &Path {
        self.0.rubies_dir()
    }
    pub fn ruby_installations(&self) -> &[RubyRuntime] {
        self.0.ruby_installations()
    }
    pub fn requested_ruby_version(&self) -> Option<&str> {
        self.0.requested_ruby_version()
    }
    pub fn gem_base_dir(&self) -> Option<&PathBuf> {
        self.0.gem_base_dir()
    }
}

pub(crate) struct ReadyRuntime(pub(super) ButlerRuntime);

impl ReadyRuntime {
    pub fn status(&self, program: &str, arguments: &[String]) -> Result<ExitStatus, ButlerError> {
        Command::new(program)
            .args(arguments)
            .status_with_validation(&self.0)
    }

    #[cfg(test)]
    pub fn empty() -> Self {
        Self(ButlerRuntime::empty(".".into(), ".".into()))
    }
}
