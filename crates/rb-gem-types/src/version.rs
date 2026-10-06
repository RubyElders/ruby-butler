use crate::ParseError;
use serde::{Deserialize, Serialize};
use std::{
    cmp::Ordering,
    fmt,
    hash::{Hash, Hasher},
    str::FromStr,
    sync::LazyLock,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Segment {
    Number(String),
    Text(String),
}

/// A RubyGems version with numeric and prerelease ordering.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct Version {
    raw: String,
    segments: Vec<Segment>,
}

impl FromStr for Version {
    type Err = ParseError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        static VALID: LazyLock<regex::Regex> = LazyLock::new(|| {
            regex::Regex::new(r"^[0-9]+(?:\.[0-9a-zA-Z]+)*(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$")
                .unwrap()
        });
        let trimmed = value.trim_matches(|c: char| c.is_ascii_whitespace());
        let trimmed = if trimmed.is_empty() { "0" } else { trimmed };
        if !VALID.is_match(trimmed) {
            return Err(ParseError::InvalidVersion(value.into()));
        }
        let raw = trimmed.replace('-', ".pre.");
        let mut segments = partition(&raw);
        let split = segments
            .iter()
            .position(|s| matches!(s, Segment::Text(_)))
            .unwrap_or(segments.len());
        let mut prerelease = segments.split_off(split);
        trim_zeros(&mut segments);
        trim_zeros(&mut prerelease);
        segments.extend(prerelease);
        Ok(Self { raw, segments })
    }
}

fn partition(value: &str) -> Vec<Segment> {
    static PARTS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"[0-9]+|[a-zA-Z]+").unwrap());
    PARTS
        .find_iter(value)
        .map(|part| {
            if part.as_str().as_bytes()[0].is_ascii_digit() {
                let digits = part.as_str().trim_start_matches('0');
                Segment::Number(if digits.is_empty() { "0" } else { digits }.into())
            } else {
                Segment::Text(part.as_str().into())
            }
        })
        .collect()
}

fn trim_zeros(segments: &mut Vec<Segment>) {
    while matches!(segments.last(), Some(Segment::Number(value)) if value == "0") {
        segments.pop();
    }
}

impl Version {
    pub fn prerelease(&self) -> bool {
        self.segments.iter().any(|s| matches!(s, Segment::Text(_)))
    }

    pub fn release(&self) -> Self {
        if !self.prerelease() {
            return self.clone();
        }
        self.release_parts().join(".").parse().unwrap()
    }

    pub fn bump(&self) -> Self {
        let mut parts = self.release_parts();
        if parts.len() > 1 {
            parts.pop();
        }
        let last = parts.last_mut().unwrap();
        let mut digits = last.as_bytes().to_vec();
        let mut carry = true;
        for digit in digits.iter_mut().rev() {
            if *digit == b'9' {
                *digit = b'0';
            } else {
                *digit += 1;
                carry = false;
                break;
            }
        }
        if carry {
            digits.insert(0, b'1');
        }
        *last = String::from_utf8(digits).unwrap();
        parts.join(".").parse().unwrap()
    }

    fn release_parts(&self) -> Vec<String> {
        partition(&self.raw)
            .into_iter()
            .take_while(|s| matches!(s, Segment::Number(_)))
            .map(|s| {
                let Segment::Number(value) = s else {
                    unreachable!()
                };
                value
            })
            .collect()
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.raw.fmt(f)
    }
}
impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl Eq for Version {}
impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.segments.hash(state);
    }
}
impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        let zero = Segment::Number("0".into());
        for i in 0..self.segments.len().max(other.segments.len()) {
            let a = self.segments.get(i).unwrap_or(&zero);
            let b = other.segments.get(i).unwrap_or(&zero);
            let order = match (a, b) {
                (Segment::Number(a), Segment::Number(b)) => {
                    a.len().cmp(&b.len()).then_with(|| a.cmp(b))
                }
                (Segment::Text(a), Segment::Text(b)) => a.cmp(b),
                (Segment::Text(_), Segment::Number(_)) => Ordering::Less,
                _ => Ordering::Greater,
            };
            if order != Ordering::Equal {
                return order;
            }
        }
        Ordering::Equal
    }
}
impl TryFrom<String> for Version {
    type Error = ParseError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        value.parse()
    }
}
impl From<Version> for String {
    fn from(value: Version) -> String {
        value.raw
    }
}
