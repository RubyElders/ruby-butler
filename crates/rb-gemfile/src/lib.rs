#![doc = include_str!("../README.md")]

mod evaluator;
mod format;
mod kdl;
mod manifest;
mod project;
mod snapshot;
mod sources;

pub use evaluator::{Evaluation, Evaluator};
pub use format::ExportFormat;
pub use manifest::{Bundle, Dependency, Manifest, Package, Plugin, Require};
pub use project::GENERATED_HEADER;
pub use snapshot::{CachedEvaluation, SnapshotCache, write_project};
