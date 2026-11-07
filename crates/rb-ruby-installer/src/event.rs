#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Phase {
    Selecting,
    Waiting,
    Downloading,
    Verifying,
    Extracting,
    Validating,
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Selecting => "Selecting Ruby",
            Self::Waiting => "Waiting for installation lock",
            Self::Downloading => "Downloading",
            Self::Verifying => "Verifying archive",
            Self::Extracting => "Extracting archive",
            Self::Validating => "Validating installation",
        }
    }

    pub fn completed_label(self) -> &'static str {
        match self {
            Self::Selecting => "Selected Ruby",
            Self::Waiting => "Acquired installation lock",
            Self::Downloading => "Downloaded",
            Self::Verifying => "Verified archive",
            Self::Extracting => "Extracted archive",
            Self::Validating => "Validated installation",
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Phase(Phase),
    Selected { version: semver::Version },
    Download { received: u64, total: u64 },
}
