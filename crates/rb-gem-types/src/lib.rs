#![doc = include_str!("../README.md")]

mod error;
mod operator;
mod package_id;
mod platform;
mod requirement;
mod version;

pub use error::ParseError;
pub use operator::Operator;
pub use package_id::PackageId;
pub use platform::Platform;
pub use requirement::Requirement;
pub use version::Version;
