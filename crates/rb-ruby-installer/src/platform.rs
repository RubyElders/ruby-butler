use anyhow::{Result, bail};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Platform {
    LinuxX86_64,
    LinuxAarch64,
    LinuxMuslX86_64,
    LinuxMuslAarch64,
    MacosX86_64,
    MacosAarch64,
}

impl Platform {
    pub fn current() -> Result<Self> {
        Self::with_inputs(&std::env::current_dir()?, &std::env::vars_os().collect())
    }

    pub fn with_inputs(
        directory: &std::path::Path,
        environment: &std::collections::BTreeMap<std::ffi::OsString, std::ffi::OsString>,
    ) -> Result<Self> {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        let musl = if os == "linux" {
            let maps = std::fs::read_to_string("/proc/self/maps").unwrap_or_default();
            if maps.contains("ld-musl") {
                true
            } else if maps.contains("libc.so.6") {
                false
            } else {
                let glibc = std::process::Command::new("getconf")
                    .current_dir(directory)
                    .env_clear()
                    .envs(environment)
                    .arg("GNU_LIBC_VERSION")
                    .output()
                    .is_ok_and(|output| output.status.success());
                if glibc {
                    false
                } else if std::fs::read_dir("/lib").is_ok_and(|entries| {
                    entries
                        .flatten()
                        .any(|entry| entry.file_name().to_string_lossy().starts_with("ld-musl-"))
                }) {
                    true
                } else {
                    bail!("Unable to determine the host Linux libc");
                }
            }
        } else {
            false
        };
        Self::for_host(os, arch, musl)
    }

    pub fn for_host(os: &str, arch: &str, musl: bool) -> Result<Self> {
        Ok(match (os, arch, musl) {
            ("linux", "x86_64", false) => Self::LinuxX86_64,
            ("linux", "aarch64", false) => Self::LinuxAarch64,
            ("linux", "x86_64", true) => Self::LinuxMuslX86_64,
            ("linux", "aarch64", true) => Self::LinuxMuslAarch64,
            ("macos", "x86_64", _) => Self::MacosX86_64,
            ("macos", "aarch64", _) => Self::MacosAarch64,
            _ => bail!(
                "Automatic Ruby installation is not supported on {os}/{arch}; install Ruby manually in the configured rubies directory"
            ),
        })
    }

    pub(crate) fn suffix(self) -> &'static str {
        match self {
            Self::LinuxX86_64 => ".x86_64_linux.tar.gz",
            Self::LinuxAarch64 => ".arm64_linux.tar.gz",
            Self::LinuxMuslX86_64 => ".x86_64_linux_musl.tar.gz",
            Self::LinuxMuslAarch64 => ".arm64_linux_musl.tar.gz",
            Self::MacosX86_64 => ".ventura.tar.gz",
            Self::MacosAarch64 => ".arm64_sonoma.tar.gz",
        }
    }
}
