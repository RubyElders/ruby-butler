use std::{error::Error, fmt};

/// Distinguishes unsatisfiable requirements from provider failures or ambiguous metadata.
#[derive(Debug)]
pub enum ResolveError {
    NoSolution(String),
    Provider {
        package: String,
        source: Box<dyn Error + Send + Sync>,
    },
    InvalidMetadata {
        package: String,
        reason: String,
    },
}

impl fmt::Display for ResolveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoSolution(explanation) => f.write_str(explanation),
            Self::Provider { package, source } => {
                write!(f, "Fetching candidates for {package} failed: {source}")
            }
            Self::InvalidMetadata { package, reason } => {
                write!(f, "Invalid candidate metadata for {package}: {reason}")
            }
        }
    }
}

impl Error for ResolveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}
