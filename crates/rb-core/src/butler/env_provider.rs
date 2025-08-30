#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct DummyProvider;
    impl EnvProvider for DummyProvider {
        fn env_vars(&self, _current_path: Option<String>) -> Vec<(String, String)> {
            vec![("FOO".into(), "BAR".into())]
        }
        fn extra_path(&self) -> Vec<PathBuf> {
            vec![PathBuf::from("/dummy/bin")]
        }
    }

    #[test]
    fn env_provider_trait_basic() {
        let p = DummyProvider;
        let envs = p.env_vars(None);
        assert_eq!(envs, vec![("FOO".into(), "BAR".into())]);
        let paths = p.extra_path();
        assert_eq!(paths, vec![PathBuf::from("/dummy/bin")]);
    }
}
use std::path::PathBuf;

pub trait EnvProvider {
    /// Returns environment variable modifications as (key, value) pairs.
    fn env_vars(&self, current_path: Option<String>) -> Vec<(String, String)>;

    /// Returns additional PATH entries.
    fn extra_path(&self) -> Vec<PathBuf>;
}