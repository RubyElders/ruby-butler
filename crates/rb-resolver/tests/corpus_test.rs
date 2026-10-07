mod corpus;
use rb_resolver::{InMemoryIndex, ResolveError, resolve};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn frozen_corpus_reproduces_bundler_target_locks() {
    let cases = corpus::cases();
    assert_eq!(cases.len(), 11);
    assert_eq!(
        cases
            .iter()
            .map(|case| &case.repository)
            .collect::<BTreeSet<_>>()
            .len(),
        11
    );
    let index = InMemoryIndex::new(corpus::packages());
    for case in cases {
        let expected = corpus::expected(&case);
        let mut request = corpus::request(&case, &expected);
        request.locked = expected
            .packages
            .iter()
            .map(|package| package.id())
            .collect();
        let selected =
            resolve(&index, &request).unwrap_or_else(|error| panic!("{}: {error}", case.name));
        let selected_ids: BTreeSet<_> = selected
            .iter()
            .map(|package| {
                [
                    package.id.name.clone(),
                    package.id.version.to_string(),
                    package.id.platform.to_string(),
                ]
            })
            .collect();
        let expected_ids = expected
            .packages
            .iter()
            .map(|package| {
                [
                    package.name.clone(),
                    package.version.clone(),
                    package.platform.clone(),
                ]
            })
            .collect();
        assert_eq!(selected_ids, expected_ids, "{}", case.name);
        corpus::validate(&request, &selected);
    }
}

#[test]
fn fresh_corpus_solutions_satisfy_every_constraint_and_ignore_provider_order() {
    let records = corpus::packages();
    let forward = InMemoryIndex::new(records.clone());
    let reverse = InMemoryIndex::new(records.into_iter().rev());
    for case in corpus::cases() {
        let request = corpus::request(&case, &corpus::expected(&case));
        let selected =
            resolve(&forward, &request).unwrap_or_else(|error| panic!("{}: {error}", case.name));
        corpus::validate(&request, &selected);
        assert_eq!(
            selected,
            resolve(&reverse, &request).unwrap(),
            "{}",
            case.name
        );
    }
}

#[test]
fn combining_real_locked_framework_versions_exposes_a_conflict() {
    let cases = corpus::cases();
    let hanami = cases.iter().find(|case| case.name == "hanami-1").unwrap();
    let sinatra = cases
        .iter()
        .find(|case| case.name == "sinatra-sequel")
        .unwrap();
    let expected = corpus::expected(hanami);
    let hanami_version = expected
        .packages
        .iter()
        .find(|package| package.name == "hanami")
        .unwrap();
    let sinatra_expected = corpus::expected(sinatra);
    let sinatra_version = sinatra_expected
        .packages
        .iter()
        .find(|package| package.name == "sinatra")
        .unwrap();
    let mut request = corpus::request(hanami, &expected);
    request.roots = BTreeMap::from([
        (
            "hanami".into(),
            corpus::requirement(&format!("= {}", hanami_version.version)),
        ),
        (
            "sinatra".into(),
            corpus::requirement(&format!("= {}", sinatra_version.version)),
        ),
    ]);
    let error = resolve(&InMemoryIndex::new(corpus::packages()), &request).unwrap_err();
    assert!(matches!(error, ResolveError::NoSolution(_)));
    assert!(error.to_string().contains("rack"), "{error}");
}

#[test]
fn padrino_failure_witness_uses_the_captured_bundler_requirement() {
    let records = corpus::packages();
    let padrino = records
        .iter()
        .find(|package| package.id.name == "padrino" && package.id.version.to_string() == "0.11.4")
        .unwrap();
    assert_eq!(padrino.dependencies["padrino-gen"].to_string(), "= 0.11.4");
    let generator = records
        .iter()
        .find(|package| {
            package.id.name == "padrino-gen" && package.id.version.to_string() == "0.11.4"
        })
        .unwrap();
    assert_eq!(generator.dependencies["bundler"].to_string(), "~> 1.0");
    let case = corpus::cases().remove(0);
    let mut request = corpus::request(&case, &corpus::expected(&case));
    request.roots = BTreeMap::from([(
        "bundler".into(),
        corpus::requirement(&format!(
            "{}, {}",
            request.roots["bundler"], generator.dependencies["bundler"]
        )),
    )]);
    let error = resolve(&InMemoryIndex::new(records), &request).unwrap_err();
    assert!(matches!(error, ResolveError::NoSolution(_)));
    assert!(error.to_string().contains("bundler"));
    let diagnostic =
        std::fs::read_to_string(corpus::root().join("padrino-conflict/bundler-error.txt")).unwrap();
    assert!(diagnostic.contains("bundler ~> 1.0"));
}
