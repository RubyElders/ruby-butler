use crate::{Lockfile, ParseError, constants::SOURCE_OPTION_ORDER};
use rb_gem_types::Requirement;
use std::fmt::Write;

pub(crate) fn render(lock: &Lockfile) -> Result<String, ParseError> {
    let mut canonical = lock.clone();
    for source in &mut canonical.sources {
        source.packages.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.version.cmp(&b.version))
                .then(a.platform.cmp(&b.platform))
        });
    }
    canonical.sources.sort_by(|a, b| {
        a.kind
            .cmp(&b.kind)
            .then(a.remotes.cmp(&b.remotes))
            .then(a.options.cmp(&b.options))
            .then_with(|| {
                a.packages
                    .iter()
                    .map(|package| package.identity())
                    .cmp(b.packages.iter().map(|package| package.identity()))
            })
    });
    canonical.platforms.sort();
    let mut out = String::new();
    for source in &canonical.sources {
        if !out.is_empty() {
            out.push('\n');
        }
        writeln!(out, "{}", source.kind.section()).unwrap();
        for remote in &source.remotes {
            writeln!(out, "  remote: {remote}").unwrap();
        }
        for key in SOURCE_OPTION_ORDER {
            if let Some(value) = source.options.get(key) {
                writeln!(out, "  {key}: {value}").unwrap();
            }
        }
        out.push_str("  specs:\n");
        for package in &source.packages {
            writeln!(out, "    {}", package.identity()).unwrap();
            for (name, requirements) in &package.dependencies {
                for requirement in requirements {
                    write_dependency(&mut out, "      ", name, requirement, false);
                }
            }
        }
    }
    out.push_str("\nPLATFORMS\n");
    for platform in &canonical.platforms {
        writeln!(out, "  {platform}").unwrap();
    }
    out.push_str("\nDEPENDENCIES\n");
    for (name, dependency) in &canonical.dependencies {
        write_dependency(
            &mut out,
            "  ",
            name,
            &dependency.requirement,
            dependency.pinned,
        );
    }
    if let Some(checksums) = &canonical.checksums {
        out.push_str("\nCHECKSUMS\n");
        for (identity, checksum) in checksums {
            write!(out, "  {identity}").unwrap();
            if let Some(checksum) = checksum {
                write!(out, " sha256={checksum}").unwrap();
            }
            out.push('\n');
        }
    }
    if let Some(ruby) = &canonical.ruby_version {
        write!(out, "\nRUBY VERSION\n   {ruby}\n").unwrap();
    }
    if let Some(bundler) = &canonical.bundled_with {
        write!(out, "\nBUNDLED WITH\n   {bundler}\n").unwrap();
    }
    if Lockfile::parse(&out)? != canonical {
        return Err(ParseError {
            line: 0,
            message: "Lockfile fields cannot be represented without loss".into(),
        });
    }
    Ok(out)
}

fn write_dependency(
    out: &mut String,
    indent: &str,
    name: &str,
    requirement: &Requirement,
    pinned: bool,
) {
    write!(out, "{indent}{name}").unwrap();
    if requirement != &Requirement::default() {
        write!(out, " ({requirement})").unwrap();
    }
    if pinned {
        out.push('!');
    }
    out.push('\n');
}
