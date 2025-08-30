use std::path::PathBuf;

pub trait RuntimeProvider {
    /// Returns the bin directory, if available.
    fn bin_dir(&self) -> Option<PathBuf>;
    /// Returns the gem directory, if available.
    fn gem_dir(&self) -> Option<PathBuf>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    struct DummyProvider;
    impl RuntimeProvider for DummyProvider {
        fn bin_dir(&self) -> Option<PathBuf> {
            Some(PathBuf::from("/dummy/bin"))
        }
        fn gem_dir(&self) -> Option<PathBuf> {
            None
        }
    }

    #[test]
    fn runtime_provider_trait_basic() {
        let p = DummyProvider;
        assert_eq!(p.bin_dir(), Some(PathBuf::from("/dummy/bin")));
        assert_eq!(p.gem_dir(), None);
    }
}
