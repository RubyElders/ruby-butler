use crate::{Package, PackageProvider, ResolveError, ResolveRequest};
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyProvider, PackageResolutionStatistics,
    PubGrubError, Ranges, Reporter,
};
use rb_gem_types::{PackageId, Platform, Requirement, Version};
use std::{cell::RefCell, collections::BTreeMap, fmt};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Name {
    Root,
    Gem(String),
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => f.write_str("<project>"),
            Self::Gem(name) => name.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct Candidate {
    version: Version,
    platform: Platform,
}

impl Candidate {
    fn package(package: &Package) -> Self {
        Self {
            version: package.id.version.clone(),
            platform: package.id.platform.clone(),
        }
    }

    fn root() -> Self {
        Self {
            version: "0".parse().unwrap(),
            platform: Platform::parse("ruby"),
        }
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} [{}]", self.version, self.platform)
    }
}

pub fn resolve<P: PackageProvider>(
    provider: &P,
    request: &ResolveRequest,
) -> Result<Vec<Package>, ResolveError> {
    let solver = Solver {
        provider,
        request,
        packages: RefCell::new(BTreeMap::new()),
    };
    let solution =
        pubgrub::resolve(&solver, Name::Root, Candidate::root()).map_err(|error| match error {
            PubGrubError::NoSolution(mut tree) => {
                tree.collapse_no_versions();
                let roots = request
                    .roots
                    .iter()
                    .map(|(name, requirement)| format!("{name} ({requirement})"))
                    .collect::<Vec<_>>()
                    .join(", ");
                ResolveError::NoSolution(format!(
                    "Dependency resolution failed for Ruby {} on {} with {roots}: {}",
                    request.ruby,
                    request.target,
                    DefaultStringReporter::report(&tree)
                ))
            }
            PubGrubError::ErrorRetrievingDependencies { source, .. }
            | PubGrubError::ErrorChoosingVersion { source, .. }
            | PubGrubError::ErrorInShouldCancel(source) => source,
        })?;
    let packages = solver.packages.borrow();
    let mut result = Vec::new();
    for (name, candidate) in solution {
        if let Name::Gem(name) = name {
            let package = packages[&name]
                .iter()
                .find(|package| Candidate::package(package) == candidate)
                .unwrap();
            result.push(package.clone());
        }
    }
    result.sort_by(|a, b| a.id.name.cmp(&b.id.name));
    Ok(result)
}

struct Solver<'a, P> {
    provider: &'a P,
    request: &'a ResolveRequest,
    packages: RefCell<BTreeMap<String, Vec<Package>>>,
}

impl<P: PackageProvider> Solver<'_, P> {
    fn fetch(&self, name: &str) -> Result<(), ResolveError> {
        if self.packages.borrow().contains_key(name) {
            return Ok(());
        }
        let mut unique = BTreeMap::<PackageId, Package>::new();
        for package in self
            .provider
            .candidates(name)
            .map_err(|source| ResolveError::Provider {
                package: name.into(),
                source: source.into(),
            })?
        {
            if package.id.name != name {
                return Err(ResolveError::InvalidMetadata {
                    package: name.into(),
                    reason: format!("provider returned {}", package.id.name),
                });
            }
            if let Some(previous) = unique.get(&package.id) {
                if previous != &package
                    || previous.id.version.to_string() != package.id.version.to_string()
                {
                    return Err(ResolveError::InvalidMetadata {
                        package: name.into(),
                        reason: format!("conflicting records for {}", package.id),
                    });
                }
            } else {
                unique.insert(package.id.clone(), package);
            }
        }
        let mut packages: Vec<_> = unique
            .into_values()
            .filter(|package| {
                package.id.platform.matches(&self.request.target)
                    && package.ruby.matches(&self.request.ruby)
            })
            .collect();
        packages.sort_by(|a, b| {
            b.id.version
                .cmp(&a.id.version)
                .then_with(|| {
                    a.id.platform
                        .score(&self.request.target)
                        .cmp(&b.id.platform.score(&self.request.target))
                })
                .then_with(|| a.id.platform.cmp(&b.id.platform))
        });
        self.packages.borrow_mut().insert(name.into(), packages);
        Ok(())
    }

    fn locked_version(&self, id: &PackageId) -> bool {
        self.request
            .locked
            .iter()
            .any(|locked| locked.name == id.name && locked.version == id.version)
    }

    fn range(
        &self,
        name: &str,
        requirement: &Requirement,
    ) -> Result<Ranges<Candidate>, ResolveError> {
        self.fetch(name)?;
        let packages = self.packages.borrow();
        let only_prereleases = packages[name]
            .iter()
            .all(|package| package.id.version.prerelease());
        let mut range = Ranges::empty();
        for package in &packages[name] {
            let prerelease_allowed = self.request.allow_prereleases
                || requirement.prerelease()
                || only_prereleases
                || self.locked_version(&package.id);
            if requirement.matches(&package.id.version)
                && (!package.id.version.prerelease() || prerelease_allowed)
            {
                range = range.union(&Ranges::singleton(Candidate::package(package)));
            }
        }
        Ok(range)
    }
}

impl<P: PackageProvider> DependencyProvider for Solver<'_, P> {
    type P = Name;
    type V = Candidate;
    type VS = Ranges<Candidate>;
    type Priority = u32;
    type M = String;
    type Err = ResolveError;

    fn prioritize(&self, _: &Name, _: &Self::VS, conflicts: &PackageResolutionStatistics) -> u32 {
        conflicts.conflict_count()
    }

    fn choose_version(
        &self,
        name: &Name,
        range: &Self::VS,
    ) -> Result<Option<Candidate>, ResolveError> {
        let Name::Gem(name) = name else {
            return Ok(Some(Candidate::root()));
        };
        self.fetch(name)?;
        let packages = self.packages.borrow();
        let eligible: Vec<_> = packages[name]
            .iter()
            .filter(|package| range.contains(&Candidate::package(package)))
            .collect();
        let locked = &self.request.locked;
        let chosen = eligible
            .iter()
            .find(|package| locked.contains(&package.id))
            .or_else(|| {
                eligible
                    .iter()
                    .find(|package| self.locked_version(&package.id))
            })
            .or_else(|| eligible.first());
        Ok(chosen.map(|package| Candidate::package(package)))
    }

    fn get_dependencies(
        &self,
        name: &Name,
        version: &Candidate,
    ) -> Result<Dependencies<Name, Self::VS, String>, ResolveError> {
        let dependencies = match name {
            Name::Root => self.request.roots.clone(),
            Name::Gem(name) => {
                self.fetch(name)?;
                self.packages.borrow()[name]
                    .iter()
                    .find(|package| Candidate::package(package) == *version)
                    .unwrap()
                    .dependencies
                    .clone()
            }
        };
        let mut result = Vec::new();
        for (name, requirement) in dependencies {
            result.push((Name::Gem(name.clone()), self.range(&name, &requirement)?));
        }
        Ok(Dependencies::Available(result.into_iter().collect()))
    }
}
