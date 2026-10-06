use std::fmt;

/// A RubyGems requirement operator, including the pessimistic `~>` bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum Operator {
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    Pessimistic,
}

impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Equal => "=",
            Self::NotEqual => "!=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Pessimistic => "~>",
        })
    }
}
