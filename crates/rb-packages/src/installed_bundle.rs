use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug)]
pub struct InstalledBundle {
    pub gem_home: PathBuf,
    pub bundle_path: PathBuf,
    pub installed: usize,
    pub reused: usize,
    pub without: Vec<String>,
    pub with: Vec<String>,
    pub gemfile: Option<PathBuf>,
}

impl InstalledBundle {
    pub fn environment(&self) -> BTreeMap<String, String> {
        let mut environment = BTreeMap::from([
            ("BUNDLE_WITHOUT".into(), self.without.join(":")),
            ("BUNDLE_WITH".into(), self.with.join(":")),
            (
                "GEM_HOME".into(),
                self.gem_home.to_string_lossy().into_owned(),
            ),
            (
                "GEM_PATH".into(),
                self.gem_home.to_string_lossy().into_owned(),
            ),
            (
                "BUNDLE_PATH".into(),
                self.bundle_path.to_string_lossy().into_owned(),
            ),
            ("BUNDLE_DISABLE_SHARED_GEMS".into(), "true".into()),
            ("BUNDLE_IGNORE_CONFIG".into(), "true".into()),
        ]);
        if let Some(gemfile) = &self.gemfile {
            environment.insert(
                "BUNDLE_GEMFILE".into(),
                gemfile.to_string_lossy().into_owned(),
            );
        }
        environment
    }
}
