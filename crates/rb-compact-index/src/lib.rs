#![doc = include_str!("../README.md")]

mod cache;
mod client;
mod constants;
mod full_index;
mod info;
mod validation;

pub use client::Client;
pub use info::{GemInfo, parse};

#[cfg(test)]
mod tests;
