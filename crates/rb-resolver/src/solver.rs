use crate::{Package, PackageProvider, ResolveError, ResolveRequest};
use pubgrub::{
    DefaultStringReporter, Dependencies, DependencyProvider, PackageResolutionStatistics,
    PubGrubError, Ranges, Reporter,
};
use rb_gem_types::{PackageId, Requirement, Version};
use std::{cell::RefCell, collections::BTreeMap, fmt};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
enum Name {
    Root,
    Gem(usize, String),
    Release(String),
}

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => f.write_str("<project>"),
            Self::Gem(_, name) | Self::Release(name) => name.fmt(f),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum Candidate {
    Root,
    Release(Version),
    Package(PackageId),
}

impl Candidate {
    fn package(package: &Package) -> Self {
        Self::Package(package.id.clone())
    }

    fn release(package: &Package) -> Self {
        Self::Release(package.id.version.clone())
    }

    fn root() -> Self {
        Self::Root
    }
}

impl fmt::Display for Candidate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => f.write_str("<project>"),
            Self::Release(version) => version.fmt(f),
            Self::Package(id) => write!(f, "{} [{}]", id.version, id.platform),
        }
    }
}

pub fn resolve<P: PackageProvider>(
    provider: &P,
    request: &ResolveRequest,
) -> Result<Vec<Package>, ResolveError> {
    resolve_targets(provider, std::slice::from_ref(request)).map(|mut results| results.remove(0))
}

/// Resolves platform-specific closures with one shared release per gem name.
pub fn resolve_targets<P: PackageProvider>(
    provider: &P,
    requests: &[ResolveRequest],
) -> Result<Vec<Vec<Package>>, ResolveError> {
    let solver = Solver {
        provider,
        requests,
        available: RefCell::new(BTreeMap::new()),
        packages: RefCell::new(BTreeMap::new()),
    };
    let solution =
        pubgrub::resolve(&solver, Name::Root, Candidate::root()).map_err(|error| match error {
            PubGrubError::NoSolution(mut tree) => {
                tree.collapse_no_versions();
                let environments = requests
                    .iter()
                    .map(|request| {
                        let roots = request
                            .roots
                            .iter()
                            .map(|(name, requirement)| format!("{name} ({requirement})"))
                            .collect::<Vec<_>>()
                            .join(", ");
                        format!("Ruby {} on {} with {roots}", request.ruby, request.target)
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                ResolveError::NoSolution(format!(
                    "Dependency resolution failed for {environments}: {}",
                    DefaultStringReporter::report(&tree)
                ))
            }
            PubGrubError::ErrorRetrievingDependencies { source, .. }
            | PubGrubError::ErrorChoosingVersion { source, .. }
            | PubGrubError::ErrorInShouldCancel(source) => source,
        })?;
    let packages = solver.packages.borrow();
    let mut result = vec![Vec::new(); requests.len()];
    for (name, candidate) in solution {
        if let Name::Gem(target, name) = name {
            let package = packages[&(target, name)]
                .iter()
                .find(|package| Candidate::package(package) == candidate)
                .unwrap();
            result[target].push(package.clone());
        }
    }
    for packages in &mut result {
        packages.sort_by(|a, b| a.id.name.cmp(&b.id.name));
    }
    Ok(result)
}

struct Solver<'a, P> {
    provider: &'a P,
    requests: &'a [ResolveRequest],
    available: RefCell<BTreeMap<String, Vec<Package>>>,
    packages: RefCell<BTreeMap<(usize, String), Vec<Package>>>,
}

impl<P: PackageProvider> Solver<'_, P> {
    fn fetch_available(&self, name: &str) -> Result<(), ResolveError> {
        if self.available.borrow().contains_key(name) {
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
                if previous != &package {
                    return Err(ResolveError::InvalidMetadata {
                        package: name.into(),
                        reason: format!("conflicting records for {}", package.id),
                    });
                }
            } else {
                unique.insert(package.id.clone(), package);
            }
        }
        self.available
            .borrow_mut()
            .insert(name.into(), unique.into_values().collect());
        Ok(())
    }

    fn fetch(&self, target: usize, name: &str) -> Result<(), ResolveError> {
        let key = (target, name.to_owned());
        if self.packages.borrow().contains_key(&key) {
            return Ok(());
        }
        self.fetch_available(name)?;
        let request = &self.requests[target];
        let mut packages: Vec<_> = self.available.borrow()[name]
            .iter()
            .filter(|package| {
                package.id.platform.matches(&request.target)
                    && package.ruby.matches(&request.ruby)
                    && (!request.force_ruby.contains(name) || package.id.platform.os == "ruby")
            })
            .cloned()
            .collect();
        packages.sort_by(|a, b| {
            b.id.version
                .cmp(&a.id.version)
                .then_with(|| {
                    a.id.platform
                        .score(&request.target)
                        .cmp(&b.id.platform.score(&request.target))
                })
                .then_with(|| a.id.platform.cmp(&b.id.platform))
                .then_with(|| b.id.version.as_str().cmp(a.id.version.as_str()))
        });
        self.packages.borrow_mut().insert(key, packages);
        Ok(())
    }

    fn locked_version(&self, target: usize, id: &PackageId) -> bool {
        self.requests[target]
            .locked
            .iter()
            .any(|locked| locked.name == id.name && locked.version.as_str() == id.version.as_str())
    }

    fn range(
        &self,
        target: usize,
        name: &str,
        requirement: &Requirement,
    ) -> Result<Ranges<Candidate>, ResolveError> {
        self.fetch(target, name)?;
        let request = &self.requests[target];
        let packages = self.packages.borrow();
        let packages = &packages[&(target, name.to_owned())];
        let only_prereleases = packages
            .iter()
            .all(|package| package.id.version.prerelease());
        let mut range = Ranges::empty();
        for package in packages {
            let prerelease_allowed = request.allow_prereleases
                || requirement.prerelease()
                || only_prereleases
                || self.locked_version(target, &package.id);
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
        match name {
            Name::Root => Ok(Some(Candidate::root())),
            Name::Release(name) => {
                self.fetch_available(name)?;
                Ok(self.available.borrow()[name]
                    .iter()
                    .map(Candidate::release)
                    .filter(|candidate| range.contains(candidate))
                    .max())
            }
            Name::Gem(target, name) => {
                let target = *target;
                self.fetch(target, name)?;
                let packages = self.packages.borrow();
                let eligible: Vec<_> = packages[&(target, name.clone())]
                    .iter()
                    .filter(|package| range.contains(&Candidate::package(package)))
                    .collect();
                let locked = &self.requests[target].locked;
                let chosen = eligible
                    .iter()
                    .find(|package| locked.contains(&package.id))
                    .or_else(|| {
                        eligible
                            .iter()
                            .find(|package| self.locked_version(target, &package.id))
                    })
                    .or_else(|| eligible.first());
                Ok(chosen.map(|package| Candidate::package(package)))
            }
        }
    }

    fn get_dependencies(
        &self,
        name: &Name,
        version: &Candidate,
    ) -> Result<Dependencies<Name, Self::VS, String>, ResolveError> {
        let mut result = Vec::new();
        match name {
            Name::Root => {
                for (target, request) in self.requests.iter().enumerate() {
                    for (name, requirement) in &request.roots {
                        result.push((
                            Name::Gem(target, name.clone()),
                            self.range(target, name, requirement)?,
                        ));
                    }
                }
            }
            Name::Release(_) => {}
            Name::Gem(target, name) => {
                self.fetch(*target, name)?;
                let package = self.packages.borrow()[&(*target, name.clone())]
                    .iter()
                    .find(|package| Candidate::package(package) == *version)
                    .unwrap()
                    .clone();
                for (dependency, requirement) in &package.dependencies {
                    result.push((
                        Name::Gem(*target, dependency.clone()),
                        self.range(*target, dependency, requirement)?,
                    ));
                }
                if self.requests.len() > 1 {
                    result.push((
                        Name::Release(name.clone()),
                        Ranges::singleton(Candidate::release(&package)),
                    ));
                }
            }
        }
        Ok(Dependencies::Available(result.into_iter().collect()))
    }
}
