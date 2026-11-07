use rb_progress::Reporter;
use std::io::{self, IsTerminal};

pub(crate) fn reporter(title: impl Into<String>) -> Reporter {
    if io::stderr().is_terminal() && io::stdout().is_terminal() {
        Reporter::with_writer(title, io::stderr())
    } else {
        Reporter::with_plain_writer(title, io::stderr())
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Completion {
    Tree,
    Compact,
}

impl Completion {
    pub(crate) fn finish(self, reporter: Reporter, summary: impl Into<String>) {
        match self {
            Self::Tree => reporter.finish_with_summary(summary),
            Self::Compact => reporter.finish_compact(summary),
        }
    }
}

pub(crate) struct PreparationReport {
    reporter: Option<Reporter>,
    completion: Completion,
}

impl PreparationReport {
    pub(crate) fn new(completion: Completion) -> Self {
        Self {
            reporter: None,
            completion,
        }
    }

    pub(crate) fn child(&mut self, title: impl Into<String>) -> Reporter {
        self.reporter
            .get_or_insert_with(|| reporter("Preparing your bundle"))
            .child(title)
    }

    pub(crate) fn take(&mut self) -> (Reporter, Completion) {
        let reporter = self
            .reporter
            .take()
            .unwrap_or_else(|| reporter("Preparing your bundle"));
        (reporter, self.completion)
    }

    pub(crate) fn finish(&mut self, summary: impl Into<String>) {
        if let Some(reporter) = self.reporter.take() {
            self.completion.finish(reporter, summary);
        }
    }

    pub(crate) fn fail(&mut self, error: &impl std::fmt::Display) {
        if let Some(reporter) = self.reporter.take() {
            reporter.handle().fail(error.to_string());
            reporter.finish();
        }
    }
}
