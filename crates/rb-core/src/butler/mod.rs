use std::path::PathBuf;

use crate::{gems::GemRuntime, ruby::RubyRuntime};

fn path_sep() -> &'static str { if cfg!(windows) { ";" } else { ":" } }

#[derive(Debug, Clone)]
pub struct ButlerRuntime {
    pub ruby: RubyRuntime,
    pub gem: GemRuntime,
    pub extra_path: Vec<PathBuf>,
}

impl ButlerRuntime {
    pub fn new(ruby: RubyRuntime, gem: GemRuntime) -> Self {
        Self { ruby, gem, extra_path: Vec::new() }
    }

    pub fn build_path(&self, current_path: Option<String>) -> String {
        let mut parts: Vec<String> = Vec::new();
        for p in &self.extra_path { parts.push(p.display().to_string()); }
        parts.push(self.gem.gem_bin.display().to_string());
        parts.push(self.ruby.bin_dir().display().to_string());
        if let Some(cp) = current_path { if !cp.is_empty() { parts.push(cp); } }
        parts.join(path_sep())
    }

    pub fn env_vars(&self, current_path: Option<String>) -> Vec<(String, String)> {
        let path = self.build_path(current_path);
        let gem_home = self.gem.gem_home.display().to_string();
        let gem_path = self.gem.gem_path
            .iter().map(|p| p.display().to_string())
            .collect::<Vec<_>>().join(path_sep());

        vec![
            ("PATH".into(), path),
            ("GEM_HOME".into(), gem_home.clone()),
            ("GEM_PATH".into(), gem_path),
        ]
    }
}

