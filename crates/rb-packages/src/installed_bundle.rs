use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
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
        self.environment_os()
            .into_iter()
            .map(|(key, value)| {
                (
                    key.to_string_lossy().into_owned(),
                    value.to_string_lossy().into_owned(),
                )
            })
            .collect()
    }

    pub fn environment_os(&self) -> BTreeMap<std::ffi::OsString, std::ffi::OsString> {
        let mut environment = BTreeMap::from([
            ("BUNDLE_WITHOUT".into(), self.without.join(":").into()),
            ("BUNDLE_WITH".into(), self.with.join(":").into()),
            ("GEM_HOME".into(), self.gem_home.clone().into_os_string()),
            ("GEM_PATH".into(), self.gem_home.clone().into_os_string()),
            (
                "BUNDLE_PATH".into(),
                self.bundle_path.clone().into_os_string(),
            ),
            ("BUNDLE_DISABLE_SHARED_GEMS".into(), "true".into()),
            ("BUNDLE_IGNORE_CONFIG".into(), "true".into()),
        ]);
        if let Some(gemfile) = &self.gemfile {
            environment.insert("BUNDLE_GEMFILE".into(), gemfile.clone().into_os_string());
        }
        environment
    }
}
