use serde::{Deserialize, Serialize};

const RUBY_PLATFORM_SCORE: u32 = 1_000_000;
const CPU_MISMATCH_PENALTY: u32 = 10;
const VERSION_MISMATCH_PENALTY: u32 = 100;

/// A gem or Ruby target platform with directional compatibility matching.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub struct Platform {
    pub cpu: Option<String>,
    pub os: String,
    pub version: Option<String>,
}

impl Platform {
    pub fn parse(value: &str) -> Self {
        use std::sync::LazyLock;
        static RULES: LazyLock<Vec<(&str, regex::Regex)>> = LazyLock::new(|| {
            [
                ("aix", r"aix-?([0-9]+)?"),
                ("cygwin", r"cygwin"),
                ("darwin", r"darwin-?([0-9]+)?"),
                ("macruby", r"^macruby-?([0-9]+(?:\.[0-9]+)*)?"),
                ("freebsd", r"freebsd-?([0-9]+)?"),
                ("java", r"^java-?([0-9]+(?:\.[0-9]+)*)?"),
                ("dalvik", r"^dalvik-?([0-9]+)?$"),
                ("dotnet", r"^dotnet-?([0-9]+(?:\.[0-9]+)*)?"),
                ("linux", r"linux-?([a-zA-Z0-9_]+)?"),
                ("mingw32", r"mingw32"),
                ("mingw", r"mingw-?([a-zA-Z0-9_]+)?"),
                ("mswin", r"(mswin[0-9]+)(?:[_-]([0-9]+))?"),
                ("netbsdelf", r"netbsdelf"),
                ("openbsd", r"openbsd-?([0-9]+\.[0-9]+)?"),
                ("solaris", r"solaris-?([0-9]+\.[0-9]+)?"),
                ("wasi", r"wasi"),
                ("custom", r"^([a-zA-Z0-9_]+_platform)-?([0-9]+)?"),
            ]
            .into_iter()
            .map(|(os, pattern)| (os, regex::Regex::new(pattern).unwrap()))
            .collect()
        });
        let ruby = value == "ruby";
        let value = value.trim_end_matches('-');
        let (cpu, os) = value
            .split_once('-')
            .map_or((None, value), |(cpu, os)| (Some(cpu), os));
        let cpu = cpu.map(|cpu| {
            if cpu
                .as_bytes()
                .windows(4)
                .any(|part| part[0] == b'i' && part[1].is_ascii_digit() && &part[2..] == b"86")
            {
                "x86"
            } else {
                cpu
            }
            .to_string()
        });
        let mut result = Self {
            cpu,
            os: "unknown".into(),
            version: None,
        };
        if ruby || os == "jruby" {
            result.os = if os == "jruby" { "java" } else { "ruby" }.into();
            return result;
        }
        for (name, pattern) in RULES.iter() {
            if let Some(captures) = pattern.captures(os) {
                if matches!(*name, "mswin" | "custom") {
                    result.os = captures[1].into();
                    result.version = captures.get(2).map(|value| value.as_str().into());
                } else {
                    result.os = (*name).into();
                    result.version = captures.get(1).map(|value| value.as_str().into());
                }
                break;
            }
        }
        if result.os == "mswin32" && result.cpu.is_none() {
            result.cpu = Some("x86".into());
        }
        result
    }

    pub fn matches(&self, target: &Self) -> bool {
        if self.os == "ruby" {
            return true;
        }
        if (self.cpu.as_deref() == Some("universal") || target.cpu.as_deref() == Some("universal"))
            && self.os.starts_with("mingw")
            && target.os.starts_with("mingw")
        {
            return true;
        }
        let cpu = self.cpu.is_none()
            || target.cpu.is_none()
            || self.cpu.as_deref() == Some("universal")
            || target.cpu.as_deref() == Some("universal")
            || self.cpu == target.cpu
            || (self.cpu.as_deref() == Some("arm")
                && target.cpu.as_deref().is_some_and(|s| s.starts_with("armv")));
        if !cpu || self.os != target.os {
            return false;
        }
        if self.os == "linux" {
            normalized_linux_version(self.version.as_deref())
                == normalized_linux_version(target.version.as_deref())
                || ["musl", "musleabi", "musleabihf"].iter().any(|prefix| {
                    target.version.as_deref()
                        == Some(
                            format!("{prefix}{}", self.version.as_deref().unwrap_or("")).as_str(),
                        )
                })
        } else {
            self.version.is_none() || target.version.is_none() || self.version == target.version
        }
    }

    pub fn score(&self, target: &Self) -> Option<u32> {
        if !self.matches(target) {
            return None;
        }
        if self == target {
            return Some(0);
        }
        if self.os == "ruby" {
            return Some(RUBY_PLATFORM_SCORE);
        }
        Some(
            1 + u32::from(self.os != target.os)
                + CPU_MISMATCH_PENALTY * u32::from(self.cpu != target.cpu)
                + VERSION_MISMATCH_PENALTY
                    * if self.version == target.version {
                        0
                    } else if self.version.is_none() {
                        1
                    } else {
                        2
                    },
        )
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(cpu) = &self.cpu {
            write!(f, "{cpu}-")?;
        }
        write!(f, "{}", self.os)?;
        if let Some(version) = &self.version {
            write!(f, "{}{version}", if self.cpu.is_some() { "-" } else { "" })?;
        }
        Ok(())
    }
}

fn normalized_linux_version(version: Option<&str>) -> &str {
    let version = version.unwrap_or("");
    let version = version.strip_prefix("gnu").unwrap_or(version);
    version
        .strip_suffix("eabihf")
        .or_else(|| version.strip_suffix("eabi"))
        .unwrap_or(version)
}
