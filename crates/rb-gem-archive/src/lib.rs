#![doc = include_str!("../README.md")]

mod archive;
mod checksums;
mod constants;
mod gem_metadata;
mod metadata;

pub use archive::read;
pub use gem_metadata::GemMetadata;
