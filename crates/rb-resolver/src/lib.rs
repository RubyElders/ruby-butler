#![doc = include_str!("../README.md")]

mod error;
mod package;
mod provider;
mod request;
mod solver;

pub use error::ResolveError;
pub use package::Package;
pub use provider::{InMemoryIndex, PackageProvider};
pub use request::ResolveRequest;
pub use solver::resolve;
