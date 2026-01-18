use polykit_core::graph::DependencyGraph;
use polykit_core::package::{Language, Package};
use proptest::prelude::*;

fn gen_package_name() -> impl Strategy<Value = String> {
    "[a-z]{3,8}"
}

fn gen_packages() -> impl Strategy<Value = Vec<Package>> {
    let names = vec!["a", "b", "c", "d", "e"];
    names
        .into_iter()
        .map(|name| {
            gen_package_name().prop_map(move |_| {
                Package::new(
                    name.to_string(),
                    Language::Rust,
                    true,
                    format!("pkg-{}", name).into(),
                    vec![],
                    vec![],
                )
            })
        })
        .collect::<Vec<_>>()
        .prop_map(|packages| packages)
}

proptest! {
    #[test]
    fn test_graph_always_valid_topological_order(packages in gen_packages()) {
        if packages.is_empty() {
            return Ok(());
        }

        if let Ok(graph) = DependencyGraph::new(packages.clone()) {
            let order = graph.topological_order();
            prop_assert!(!order.is_empty());
            prop_assert_eq!(order.len(), packages.len());
        }
    }

    #[test]
    fn test_graph_no_duplicates_in_order(packages in gen_packages()) {
        if packages.is_empty() {
            return Ok(());
        }

        if let Ok(graph) = DependencyGraph::new(packages) {
            let order = graph.topological_order();
            let mut seen = std::collections::HashSet::new();
            for pkg in order {
                prop_assert!(seen.insert(pkg.clone()), "Duplicate package in order: {}", pkg);
            }
        }
    }
}
