use crate::validation::{complete, validate};
use crate::{Distribution, Event, Installer, Phase, Platform, Release, Request};
use anyhow::{Context, Result, ensure};
use fs2::FileExt;
use semver::Version;
use serde::Serialize;
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Installation {
    pub version: Version,
    pub path: PathBuf,
    pub installed: bool,
}

#[derive(Serialize)]
struct Receipt<'a> {
    version: &'a str,
    release: &'a str,
    url: &'a str,
    sha256: &'a str,
}

impl Installer {
    pub fn install(
        &self,
        request: &Request,
        platform: Platform,
        directory: &Path,
        mut report: impl FnMut(Event),
    ) -> Result<Installation> {
        if let Some(version) = request.exact_version() {
            let path = directory.join(format!("ruby-{version}"));
            if complete(&path) {
                report(Event::Selected {
                    version: version.clone(),
                });
                return Ok(Installation {
                    version,
                    path,
                    installed: false,
                });
            }
        }
        report(Event::Phase(Phase::Selecting));
        let release = self.catalog()?;
        let distribution = release.select(request, platform)?;
        report(Event::Selected {
            version: distribution.version.clone(),
        });
        self.install_distribution(&release, distribution, directory, &mut report)
    }

    fn install_distribution(
        &self,
        release: &Release,
        distribution: Distribution,
        directory: &Path,
        report: &mut dyn FnMut(Event),
    ) -> Result<Installation> {
        let version = distribution.version;
        fs::create_dir_all(directory)?;
        let locks = directory.join(".locks");
        fs::create_dir_all(&locks)?;
        let lock = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(locks.join(format!("ruby-{version}.lock")))?;
        match lock.try_lock_exclusive() {
            Ok(()) => (),
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                report(Event::Phase(Phase::Waiting));
                lock.lock_exclusive()?;
            }
            Err(error) => return Err(error.into()),
        }
        let destination = directory.join(format!("ruby-{version}"));
        if complete(&destination) {
            return Ok(Installation {
                version,
                path: destination,
                installed: false,
            });
        }
        ensure!(
            !destination.exists(),
            "Incomplete Ruby installation at {}; move it aside before retrying",
            destination.display()
        );
        let digest = self.checksum(release, &distribution.asset)?;
        let archive = self.download(&distribution.asset, &digest, report)?;
        report(Event::Phase(Phase::Extracting));
        let stage = tempfile::Builder::new()
            .prefix(".ruby-install-")
            .tempdir_in(directory)?;
        let extracted = crate::archive::extract(&archive, stage.path())?;
        report(Event::Phase(Phase::Validating));
        validate(&extracted, &version, &self.directory, &self.environment)?;
        let version_text = version.to_string();
        let receipt = Receipt {
            version: &version_text,
            release: &distribution.release,
            url: &distribution.asset.browser_download_url,
            sha256: &digest,
        };
        fs::write(
            extracted.join(".rb-install.json"),
            serde_json::to_vec_pretty(&receipt)?,
        )?;
        fs::rename(&extracted, &destination).context("Unable to publish the Ruby installation")?;
        Ok(Installation {
            version,
            path: destination,
            installed: true,
        })
    }
}
