use crate::{
    Lockfile, ParseError, Source, SourceKind,
    constants::{ENVIRONMENT_INDENT, SECTION_INDENT, SOURCE_OPTION_ORDER},
};
use rb_gem_types::Requirement;
use std::fmt::Write;

pub(crate) fn render(lock: &Lockfile) -> Result<String, ParseError> {
    let mut canonical = lock.clone();
    for source in &mut canonical.sources {
        source.packages.sort_by_cached_key(|package| {
            format!("{}-{}", package.name, package.archive_version())
        });
        for package in &mut source.packages {
            for requirements in package.dependencies.values_mut() {
                for requirement in requirements {
                    *requirement = canonical_requirement(requirement);
                }
            }
        }
    }
    for dependency in canonical.dependencies.values_mut() {
        dependency.requirement = canonical_requirement(&dependency.requirement);
    }
    canonical.sources.sort_by(|a, b| {
        (a.kind == SourceKind::Registry)
            .cmp(&(b.kind == SourceKind::Registry))
            .then_with(|| source_identifier(a).cmp(&source_identifier(b)))
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
    let indent = if canonical
        .bundled_with
        .as_ref()
        .is_some_and(|version| version >= &"4.0.0".parse().unwrap())
    {
        SECTION_INDENT
    } else {
        ENVIRONMENT_INDENT
    };
    if let Some(ruby) = &canonical.ruby_version {
        write!(out, "\nRUBY VERSION\n{:indent$}{ruby}\n", "").unwrap();
    }
    if let Some(bundler) = &canonical.bundled_with {
        write!(out, "\nBUNDLED WITH\n{:indent$}{bundler}\n", "").unwrap();
    }
    if Lockfile::parse(&out)? != canonical {
        return Err(ParseError {
            line: 0,
            message: "Lockfile fields cannot be represented without loss".into(),
        });
    }
    Ok(out)
}

fn source_identifier(source: &Source) -> String {
    match source.kind {
        SourceKind::Path => format!("source at `{}`", source.remotes.join(", ")),
        _ => source.remotes.join(", "),
    }
}

fn canonical_requirement(requirement: &Requirement) -> Requirement {
    let mut constraints = requirement
        .constraints()
        .map(|(operator, version)| format!("{operator} {version}"))
        .collect::<Vec<_>>();
    constraints.sort_by(|a, b| b.cmp(a));
    constraints
        .join(", ")
        .parse()
        .expect("existing lockfile requirements")
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
