#![doc = include_str!("../README.md")]

mod events;
mod plain;
mod render;
mod reporter;
mod state;
#[cfg(feature = "rb-task")]
mod task;
mod terminal;

pub use events::ProgressEvent;
pub use reporter::{ProgressHandle, Reporter};
pub use terminal::format_duration;
