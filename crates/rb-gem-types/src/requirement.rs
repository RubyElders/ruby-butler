use crate::{Operator, ParseError, Version};
use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr, sync::LazyLock};

#[derive(Clone, Debug, Eq, PartialEq)]
struct Constraint {
    operator: Operator,
    version: Version,
}

/// A conjunction of RubyGems version constraints; prerelease selection is caller policy.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Requirement {
    constraints: Vec<Constraint>,
}

impl FromStr for Requirement {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        static PATTERN: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(r"^[ \t\r\n\x0B\x0C]*(~>|>=|<=|!=|=|>|<)?[ \t\r\n\x0B\x0C]*(\S+)[ \t\r\n\x0B\x0C]*$").unwrap()
        });
        let mut constraints = Vec::new();
        for part in value.split([',', '&']) {
            let captures = PATTERN
                .captures(part)
                .ok_or_else(|| ParseError::InvalidRequirement(value.into()))?;
            let version = captures[2]
                .parse()
                .map_err(|_| ParseError::InvalidRequirement(value.into()))?;
            constraints.push(Constraint {
                operator: match captures.get(1).map_or("=", |m| m.as_str()) {
                    "=" => Operator::Equal,
                    "!=" => Operator::NotEqual,
                    ">" => Operator::Greater,
                    ">=" => Operator::GreaterEqual,
                    "<" => Operator::Less,
                    "<=" => Operator::LessEqual,
                    "~>" => Operator::Pessimistic,
                    _ => unreachable!(),
                },
                version,
            });
        }
        Ok(Self { constraints })
    }
}

impl Requirement {
    pub fn constraints(&self) -> impl ExactSizeIterator<Item = (Operator, &Version)> {
        self.constraints
            .iter()
            .map(|constraint| (constraint.operator, &constraint.version))
    }

    pub fn matches(&self, version: &Version) -> bool {
        self.constraints.iter().all(|constraint| {
            let required = &constraint.version;
            match constraint.operator {
                Operator::Equal => version == required,
                Operator::NotEqual => version != required,
                Operator::GreaterEqual => version >= required,
                Operator::Greater => version > required,
                Operator::LessEqual => version <= required,
                Operator::Less => version < required,
                Operator::Pessimistic => version >= required && version.release() < required.bump(),
            }
        })
    }

    pub fn prerelease(&self) -> bool {
        self.constraints
            .iter()
            .any(|constraint| constraint.version.prerelease())
    }
}

impl Default for Requirement {
    fn default() -> Self {
        ">= 0".parse().unwrap()
    }
}
impl fmt::Display for Requirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, constraint) in self.constraints.iter().enumerate() {
            if index > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{} {}", constraint.operator, constraint.version)?;
        }
        Ok(())
    }
}
impl TryFrom<String> for Requirement {
    type Error = ParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl From<Requirement> for String {
    fn from(value: Requirement) -> String {
        value.to_string()
    }
}
