use std::{ffi::OsString, path::PathBuf, time::Duration};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildStage {
    Configure,
    Compile,
    Install,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug)]
pub enum BuildEvent {
    Started {
        extension: PathBuf,
        stage: BuildStage,
        program: OsString,
        arguments: Vec<OsString>,
    },
    Output {
        extension: PathBuf,
        stage: BuildStage,
        stream: OutputStream,
        bytes: Vec<u8>,
    },
    Finished {
        extension: PathBuf,
        stage: BuildStage,
        elapsed: Duration,
    },
}
