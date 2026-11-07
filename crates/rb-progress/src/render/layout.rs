use crate::terminal::{fit_terminal, wrap_terminal};

pub(super) struct TreeLayout(String);

impl TreeLayout {
    pub(super) fn root() -> Self {
        Self("│  ".into())
    }

    pub(super) fn child(&self) -> Self {
        Self(format!("{}│  ", self.0))
    }

    pub(super) fn row(&self, branch: &str, text: &str) -> String {
        fit_terminal(&format!("{}{branch} {text}", self.0))
    }

    pub(super) fn preview(&self, text: &str) -> String {
        let line = fit_terminal(&format!("{}   {text}", self.0));
        format!("\x1b[2m{line}\x1b[22m")
    }

    pub(super) fn error(&self, branch: &str, text: &str) -> Vec<String> {
        wrap_terminal(
            text,
            &format!("{}{branch} × ", self.0),
            &format!("{}     ", self.0),
        )
    }
}
