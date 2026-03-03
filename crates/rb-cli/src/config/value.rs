use std::fmt;

/// Source of a configuration value
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSource {
    /// Not yet resolved - will be determined during environment discovery
    Unresolved,
    /// Built-in default value
    Default,
    /// From environment variable
    EnvVar,
    /// From configuration file (rb.toml or rb.kdl)
    ConfigFile,
    /// From CLI argument
    Cli,
    /// Automatically resolved during environment discovery
    Resolved,
}

impl fmt::Display for ConfigSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigSource::Unresolved => write!(f, "unresolved"),
            ConfigSource::Default => write!(f, "default"),
            ConfigSource::EnvVar => write!(f, "environment"),
            ConfigSource::ConfigFile => write!(f, "config file"),
            ConfigSource::Cli => write!(f, "CLI argument"),
            ConfigSource::Resolved => write!(f, "auto-resolved"),
        }
    }
}

/// A configuration value with its source tracked
#[derive(Debug, Clone)]
pub struct ConfigValue<T> {
    pub value: T,
    pub source: ConfigSource,
}

impl<T> ConfigValue<T> {
    pub fn new(value: T, source: ConfigSource) -> Self {
        Self { value, source }
    }

    pub fn default_value(value: T) -> Self {
        Self {
            value,
            source: ConfigSource::Default,
        }
    }

    pub fn from_env(value: T) -> Self {
        Self {
            value,
            source: ConfigSource::EnvVar,
        }
    }

    pub fn from_file(value: T) -> Self {
        Self {
            value,
            source: ConfigSource::ConfigFile,
        }
    }

    pub fn from_cli(value: T) -> Self {
        Self {
            value,
            source: ConfigSource::Cli,
        }
    }

    /// Create an unresolved value (placeholder for later resolution)
    pub fn unresolved(value: T) -> Self {
        Self {
            value,
            source: ConfigSource::Unresolved,
        }
    }

    /// Mark value as resolved during environment discovery
    pub fn resolved(value: T) -> Self {
        Self {
            value,
            source: ConfigSource::Resolved,
        }
    }

    pub fn is_unresolved(&self) -> bool {
        self.source == ConfigSource::Unresolved
    }

    /// Check if this value has been explicitly set (not unresolved or default)
    pub fn is_explicit(&self) -> bool {
        matches!(
            self.source,
            ConfigSource::Cli | ConfigSource::ConfigFile | ConfigSource::EnvVar
        )
    }

    /// Update this value and mark as resolved (if it was unresolved)
    pub fn resolve(&mut self, new_value: T) {
        if self.source == ConfigSource::Unresolved {
            self.value = new_value;
            self.source = ConfigSource::Resolved;
        }
    }

    /// Update this value and mark as resolved, returning the old value
    pub fn resolve_replace(&mut self, new_value: T) -> T {
        let old_value = std::mem::replace(&mut self.value, new_value);
        if self.source == ConfigSource::Unresolved {
            self.source = ConfigSource::Resolved;
        }
        old_value
    }

    pub fn get(&self) -> &T {
        &self.value
    }

    pub fn get_mut(&mut self) -> &mut T {
        &mut self.value
    }

    pub fn into_inner(self) -> T {
        self.value
    }

    pub fn map<U, F>(self, f: F) -> ConfigValue<U>
    where
        F: FnOnce(T) -> U,
    {
        ConfigValue {
            value: f(self.value),
            source: self.source,
        }
    }

    /// Update value only if new source has higher priority
    /// Priority: CLI > ConfigFile > EnvVar > Default
    pub fn merge_with(&mut self, other: ConfigValue<T>) {
        let self_priority = self.source.priority();
        let other_priority = other.source.priority();

        if other_priority > self_priority {
            *self = other;
        }
    }
}

impl ConfigSource {
    /// Get priority of this source (higher = takes precedence)
    fn priority(self) -> u8 {
        match self {
            ConfigSource::Unresolved => 0,
            ConfigSource::Default => 1,
            ConfigSource::EnvVar => 2,
            ConfigSource::ConfigFile => 3,
            ConfigSource::Resolved => 4,
            ConfigSource::Cli => 5,
        }
    }

    pub fn is_default(self) -> bool {
        self == ConfigSource::Default
    }
}

impl<T: Default> Default for ConfigValue<T> {
    fn default() -> Self {
        Self::default_value(T::default())
    }
}

impl<T: fmt::Display> fmt::Display for ConfigValue<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (from {})", self.value, self.source)
    }
}
