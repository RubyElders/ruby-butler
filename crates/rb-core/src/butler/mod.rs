#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct DummyProvider;
    impl env_provider::EnvProvider for DummyProvider {
        fn env_vars(&self, _current_path: Option<String>) -> Vec<(String, String)> {
            vec![("FOO".into(), "BAR".into())]
        }
        fn extra_path(&self) -> Vec<PathBuf> {
            vec![PathBuf::from("/dummy/bin")]
        }
    }

    #[test]
    fn butler_runtime_composes_envs_and_paths() {
        let p1 = Box::new(DummyProvider);
        let p2 = Box::new(DummyProvider);
        let butler = ButlerRuntime::new(vec![p1, p2]);
        let envs = butler.env_vars(Some("/usr/bin".to_string()));
        let mut found_path = false;
        let mut found_foo = 0;
        for (k, v) in envs {
            match k.as_str() {
                "PATH" => {
                    assert!(v.contains("/dummy/bin"));
                    assert!(v.contains("/usr/bin"));
                    found_path = true;
                }
                "FOO" => {
                    assert_eq!(v, "BAR");
                    found_foo += 1;
                }
                _ => {}
            }
        }
        assert!(found_path);
        assert_eq!(found_foo, 2);
    }
}
use std::path::PathBuf;

// ...existing code...
pub mod env_provider;
use env_provider::EnvProvider;

pub fn path_sep() -> &'static str { if cfg!(windows) { ";" } else { ":" } }

pub struct ButlerRuntime {
    pub providers: Vec<Box<dyn EnvProvider>>,
    pub extra_path: Vec<PathBuf>,
}

impl ButlerRuntime {
    pub fn new(providers: Vec<Box<dyn EnvProvider>>) -> Self {
        Self { providers, extra_path: Vec::new() }
    }

    pub fn build_path(&self, current_path: Option<String>) -> String {
        let mut parts: Vec<String> = Vec::new();
        for provider in &self.providers {
            for p in provider.extra_path() {
                parts.push(p.display().to_string());
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

    pub fn env_vars(&self, current_path: Option<String>) -> Vec<(String, String)> {
        let mut envs = Vec::new();
        for provider in &self.providers {
            envs.extend(provider.env_vars(current_path.clone()));
        }
        envs.push(("PATH".into(), self.build_path(current_path)));
        envs
    }
}

