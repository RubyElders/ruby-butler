use crate::receipt::Receipt;
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

pub(crate) type Slots = BTreeMap<String, Arc<Mutex<Slot>>>;

pub(crate) struct Slot {
    pub(crate) directory: PathBuf,
    pub(crate) archive: Option<PathBuf>,
    pub(crate) metadata: Option<rb_gem_archive::GemMetadata>,
    pub(crate) staging: Option<tempfile::TempDir>,
    pub(crate) reused: bool,
    pub(crate) receipt: Receipt,
    pub(crate) gem: Option<rb_microgems::InstalledGem>,
    pub(crate) dependencies: std::collections::BTreeSet<String>,
}
