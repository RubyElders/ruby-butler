pub mod common;
use common::{package, request};
use rb_resolver::{InMemoryIndex, ResolveError, resolve};

#[test]
fn cyclic_graphs_agree_with_exhaustive_assignment_search() {
    let names = ["a", "b", "c"];
    let mut solved = 0;
    let mut impossible = 0;
    for graph in 0_u8..64 {
        let mut packages = Vec::new();
        for (index, name) in names.iter().enumerate() {
            for version in 0..2 {
                let required = 1 + ((graph >> (2 * index + version)) & 1);
                packages.push(package(
                    name,
                    &(version + 1).to_string(),
                    &[(names[(index + 1) % 3], &format!("= {required}"))],
                ));
            }
        }
        let feasible: Vec<_> = (0_u8..8)
            .filter(|assignment| {
                (0..3).all(|index| {
                    let selected = (assignment >> index) & 1;
                    let dependency = (assignment >> ((index + 1) % 3)) & 1;
                    dependency == ((graph >> (2 * index + usize::from(selected))) & 1)
                })
            })
            .collect();
        let result = resolve(
            &InMemoryIndex::new(packages),
            &request(&[("a", ">= 0"), ("b", ">= 0"), ("c", ">= 0")]),
        );
        match result {
            Ok(packages) => {
                assert_eq!(packages.len(), 3);
                let assignment =
                    packages
                        .iter()
                        .enumerate()
                        .fold(0_u8, |assignment, (index, package)| {
                            assert_eq!(package.id.name, names[index]);
                            assignment
                                | ((package.id.version.to_string().parse::<u8>().unwrap() - 1)
                                    << index)
                        });
                assert!(
                    feasible.contains(&assignment),
                    "invalid assignment {assignment} for graph {graph}"
                );
                solved += 1;
            }
            Err(ResolveError::NoSolution(_)) => {
                assert!(
                    feasible.is_empty(),
                    "missed assignments {feasible:?} for graph {graph}"
                );
                impossible += 1;
            }
            Err(error) => panic!("unexpected error for graph {graph}: {error}"),
        }
    }
    assert!(solved > 0);
    assert!(impossible > 0);
    assert_eq!(solved + impossible, 64);
}
