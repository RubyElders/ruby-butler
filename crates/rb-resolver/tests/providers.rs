mod common;
use common::{names, package, request};
use rb_resolver::{InMemoryIndex, Package, PackageProvider, ResolveError, resolve};
use std::{cell::RefCell, collections::BTreeMap, error::Error, io};

struct Counting {
    index: InMemoryIndex,
    calls: RefCell<BTreeMap<String, usize>>,
}

impl PackageProvider for Counting {
    type Error = io::Error;
    fn candidates(&self, name: &str) -> Result<Vec<Package>, Self::Error> {
        *self.calls.borrow_mut().entry(name.into()).or_default() += 1;
        Ok(self.index.candidates(name).unwrap())
    }
}

#[test]
fn each_reachable_gem_is_fetched_once_even_during_backtracking() {
    let provider = Counting {
        index: InMemoryIndex::new([
            package("a", "2", &[("shared", ">= 2")]),
            package("a", "1", &[("shared", "< 2")]),
            package("b", "1", &[("shared", "< 2")]),
            package("shared", "1", &[]),
            package("shared", "2", &[]),
        ]),
        calls: RefCell::new(BTreeMap::new()),
    };
    let result = resolve(&provider, &request(&[("a", ">= 0"), ("b", ">= 0")])).unwrap();
    assert_eq!(result[0].id.version.to_string(), "1");
    assert_eq!(
        *provider.calls.borrow(),
        BTreeMap::from([("a".into(), 1), ("b".into(), 1), ("shared".into(), 1)])
    );
}

#[test]
fn provider_order_does_not_change_selection() {
    let records = vec![
        package("app", "1", &[]),
        package("app", "2", &[]),
        package("app", "3", &[]),
    ];
    let input = request(&[("app", ">= 0")]);
    let expected = names(resolve(&InMemoryIndex::new(records.clone()), &input).unwrap());
    assert_eq!(expected["app"], "3");
    assert_eq!(
        names(resolve(&InMemoryIndex::new(records.into_iter().rev()), &input).unwrap()),
        expected
    );
}

#[test]
fn identical_duplicates_are_deduplicated_and_conflicting_duplicates_rejected() {
    let candidate = package("app", "1", &[]);
    let input = request(&[("app", ">= 0")]);
    assert_eq!(
        resolve(
            &InMemoryIndex::new([candidate.clone(), candidate.clone()]),
            &input
        )
        .unwrap()
        .len(),
        1
    );
    let mut different = candidate.clone();
    different
        .dependencies
        .insert("other".into(), ">= 0".parse().unwrap());
    assert!(matches!(
        resolve(&InMemoryIndex::new([candidate, different]), &input),
        Err(ResolveError::InvalidMetadata { .. })
    ));
}

struct Failing;
impl PackageProvider for Failing {
    type Error = io::Error;
    fn candidates(&self, _: &str) -> Result<Vec<Package>, Self::Error> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "credentials unavailable",
        ))
    }
}

#[test]
fn provider_errors_keep_the_package_name_and_original_cause() {
    let error = resolve(&Failing, &request(&[("app", ">= 0")])).unwrap_err();
    assert!(matches!(error, ResolveError::Provider { .. }));
    assert!(error.to_string().contains("app"));
    assert!(error.to_string().contains("credentials unavailable"));
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::PermissionDenied
    );
}

struct Mismatched;
impl PackageProvider for Mismatched {
    type Error = io::Error;
    fn candidates(&self, _: &str) -> Result<Vec<Package>, Self::Error> {
        Ok(vec![package("other", "1", &[])])
    }
}

#[test]
fn mismatched_provider_names_are_errors_not_unsatisfiable_requirements() {
    let error = resolve(&Mismatched, &request(&[("app", ">= 0")])).unwrap_err();
    assert!(matches!(error, ResolveError::InvalidMetadata { .. }));
    assert!(error.to_string().contains("provider returned other"));
}

#[test]
fn equal_versions_with_different_archive_spellings_are_rejected() {
    let index = InMemoryIndex::new([package("app", "1", &[]), package("app", "1.0", &[])]);
    let error = resolve(&index, &request(&[("app", ">= 0")])).unwrap_err();
    assert!(matches!(error, ResolveError::InvalidMetadata { .. }));
}

#[test]
fn empty_roots_do_not_call_the_provider() {
    assert!(resolve(&Failing, &request(&[])).unwrap().is_empty());
}

struct BoxedFailure;
impl PackageProvider for BoxedFailure {
    type Error = Box<dyn Error + Send + Sync>;
    fn candidates(&self, _: &str) -> Result<Vec<Package>, Self::Error> {
        Err(io::Error::new(io::ErrorKind::NotFound, "registry cache missing").into())
    }
}

#[test]
fn boxed_provider_errors_preserve_the_underlying_error() {
    let error = resolve(&BoxedFailure, &request(&[("app", ">= 0")])).unwrap_err();
    assert_eq!(
        error
            .source()
            .unwrap()
            .downcast_ref::<io::Error>()
            .unwrap()
            .kind(),
        io::ErrorKind::NotFound
    );
    assert!(error.to_string().contains("registry cache missing"));
}
