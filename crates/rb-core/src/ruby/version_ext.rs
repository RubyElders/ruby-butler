//! Extension methods for semver::Version to support Ruby-specific version formats

use semver::Version;

/// Extension trait for Ruby ABI version formatting
///
/// Ruby uses a "ruby_version" (RbConfig::CONFIG["ruby_version"]) which represents
/// the ABI compatibility level. This is the major.minor version with patch always 0.
/// For example, Ruby 3.3.7 has ruby_version "3.3.0", and Ruby 3.4.5 has "3.4.0".
///
/// This is used for:
/// - Library installation paths (e.g., `/usr/lib/ruby/3.3.0/`)
/// - Bundler vendor directories (e.g., `.rb/vendor/bundler/ruby/3.3.0/`)
/// - Native extension compatibility checks
pub trait RubyVersionExt {
    /// Returns the Ruby ABI version string (major.minor.0)
    ///
    /// This corresponds to RbConfig::CONFIG["ruby_version"] in Ruby.
    ///
    /// # Examples
    ///
    /// ```
    /// use semver::Version;
    /// use rb_core::ruby::RubyVersionExt;
    ///
    /// let v = Version::new(3, 3, 7);
    /// assert_eq!(v.ruby_abi_version(), "3.3.0");
    ///
    /// let v = Version::new(3, 4, 5);
    /// assert_eq!(v.ruby_abi_version(), "3.4.0");
    /// ```
    fn ruby_abi_version(&self) -> String;
}

impl RubyVersionExt for Version {
    fn ruby_abi_version(&self) -> String {
        format!("{}.{}.0", self.major, self.minor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ruby_abi_version() {
        let v = Version::new(3, 3, 7);
        assert_eq!(v.ruby_abi_version(), "3.3.0");

        let v = Version::new(3, 4, 5);
        assert_eq!(v.ruby_abi_version(), "3.4.0");

        let v = Version::new(2, 7, 8);
        assert_eq!(v.ruby_abi_version(), "2.7.0");

        let v = Version::new(3, 3, 0);
        assert_eq!(v.ruby_abi_version(), "3.3.0");
    }
}
