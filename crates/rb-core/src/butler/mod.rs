#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct DummyProvider;
    impl crate::butler::runtime_provider::RuntimeProvider for DummyProvider {
        fn bin_dir(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/dummy/bin"))
        }
        fn gem_dir(&self) -> Option<PathBuf> {
            None
        }
    }

    #[test]
    fn butler_runtime_composes_envs_and_paths() {
    let p1 = Box::new(DummyProvider);
    let p2 = Box::new(DummyProvider);
    let butler = ButlerRuntime::new(vec![p1, p2]);
    let path = butler.build_path(Some("/usr/bin".to_string()));
    assert!(path.contains("/dummy/bin"));
    assert!(path.contains("/usr/bin"));
    }
}
use std::path::PathBuf;

// ...existing code...
pub mod runtime_provider;
use runtime_provider::RuntimeProvider;

pub fn path_sep() -> &'static str { if cfg!(windows) { ";" } else { ":" } }

pub struct ButlerRuntime {
    pub providers: Vec<Box<dyn RuntimeProvider>>,
    pub extra_path: Vec<PathBuf>,
}

impl ButlerRuntime {
    pub fn new(providers: Vec<Box<dyn RuntimeProvider>>) -> Self {
        Self { providers, extra_path: Vec::new() }
    }

    pub fn build_path(&self, current_path: Option<String>) -> String {
        let mut parts: Vec<String> = Vec::new();
        for provider in &self.providers {
            if let Some(bin) = provider.bin_dir() {
                parts.push(bin.display().to_string());
            }
        }
        for p in &self.extra_path {
            parts.push(p.display().to_string());
        }
        if let Some(cp) = current_path {
            if !cp.is_empty() {
                parts.push(cp);
            }
        }
        parts.join(path_sep())
    }
}

