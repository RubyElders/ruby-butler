use std::fmt;

/// Reports malformed gem versions and requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseError {
    InvalidVersion(String),
    InvalidRequirement(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidVersion(value) => write!(formatter, "Invalid gem version: {value:?}"),
            Self::InvalidRequirement(value) => {
                write!(formatter, "Invalid gem requirement: {value:?}")
            }
        }
    }
}

impl std::error::Error for ParseError {}
