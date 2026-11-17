//! Path counts on small graphs: E0E-2.4, E0E-4.1 and E0E-4.2, checked against an enumeration of
//! every path.

use super::super::output::{
    DatasetIndex, DatasetPair, Edge, GraphDataset, PathCount, Reachable, RelationshipGraph,
};
use super::super::paths::reachability;
use crate::names::Name;
use crate::pointer::JsonPointer;

/// A graph with `size` datasets and the edges `edges`, as (from, to) indices.
fn graph(size: usize, edges: &[(usize, usize)]) -> RelationshipGraph {
    RelationshipGraph {
        datasets: (0..size)
            .map(|index| GraphDataset {
                name: Name::declared(&format!("d{index}")),
                pointer: JsonPointer::root().member("datasets").item(index),
            })
            .collect(),
        edges: edges
            .iter()
            .enumerate()
            .map(|(index, &(from, to))| Edge {
                name: Name::declared(&format!("r{index}")),
                pointer: JsonPointer::root().member("relationships").item(index),
                from: DatasetIndex(from),
                to: DatasetIndex(to),
                one_to_one: false,
            })
            .collect(),
    }
}

/// The pairs with more than one path, as (from, to) indices.
fn several(graph: &RelationshipGraph) -> Vec<(usize, usize)> {
    reachability(graph)
        .1
        .iter()
        .map(|DatasetPair { from, to }| (from.0, to.0))
        .collect()
}

/// The number of paths from `from` to every dataset, capped at 2, by enumerating every path
/// that never repeats a dataset (E0E-2.4).
fn enumerated(graph: &RelationshipGraph, from: usize) -> Vec<usize> {
    let size = graph.datasets.len();
    let mut counts = vec![0; size];
    let mut visited = vec![false; size];
    visited[from] = true;
    walk(graph, from, &mut visited, &mut counts);
    counts.iter().map(|count| (*count).min(2)).collect()
}

fn walk(graph: &RelationshipGraph, at: usize, visited: &mut [bool], counts: &mut [usize]) {
    for edge in graph.edges.iter().filter(|edge| edge.from.0 == at) {
        let next = edge.to.0;
        if visited[next] {
            continue;
        }
        counts[next] += 1;
        visited[next] = true;
        walk(graph, next, visited, counts);
        visited[next] = false;
    }
}

#[test]
fn e0e_4_2_a_diamond_reports_its_pair_once() {
    let diamond = graph(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
    assert_eq!(several(&diamond), [(0, 3)]);
}

#[test]
fn e0e_4_1_two_relationships_between_the_same_datasets_are_two_paths() {
    let calendar = graph(3, &[(0, 1), (0, 1), (1, 2)]);
    assert_eq!(several(&calendar), [(0, 1), (0, 2)]);
}

#[test]
fn e0e_2_4_cycles_never_repeat_a_dataset() {
    let two_cycle = graph(2, &[(0, 1), (1, 0)]);
    assert!(several(&two_cycle).is_empty());
    let entered_cycle = graph(3, &[(0, 1), (1, 2), (2, 1)]);
    assert!(several(&entered_cycle).is_empty());
    let entered_twice = graph(3, &[(0, 1), (0, 2), (1, 2), (2, 1)]);
    assert_eq!(several(&entered_twice), [(0, 1), (0, 2)]);
}

#[test]
fn e0e_6_reachability_lists_each_reached_dataset_in_graph_order() {
    let chain = graph(4, &[(2, 1), (1, 0), (0, 3)]);
    let (reachable, _) = reachability(&chain);
    let one = |index| Reachable {
        dataset: DatasetIndex(index),
        paths: PathCount::One,
    };
    assert_eq!(reachable[2], [one(0), one(1), one(3)]);
    assert!(reachable[3].is_empty());
}

/// A deterministic sequence of pseudo-random numbers (a linear congruential generator).
struct Numbers(u64);

impl Numbers {
    fn below(&mut self, bound: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let high = usize::try_from(self.0 >> 33).expect("31 bits fit in usize");
        high % bound
    }
}

#[test]
fn e0e_4_2_path_counts_match_an_enumeration_of_every_path() {
    let mut numbers = Numbers(34);
    for _ in 0..2_000 {
        let size = 1 + numbers.below(7);
        let edge_count = numbers.below(3 * size);
        let mut edges = Vec::new();
        for _ in 0..edge_count {
            let (from, to) = (numbers.below(size), numbers.below(size));
            if from != to {
                edges.push((from, to));
            }
        }
        let random = graph(size, &edges);
        let (reachable, pairs) = reachability(&random);
        for (from, reached) in reachable.iter().enumerate() {
            let expected: Vec<Reachable> = enumerated(&random, from)
                .iter()
                .enumerate()
                .filter(|(to, count)| *to != from && **count > 0)
                .map(|(to, count)| Reachable {
                    dataset: DatasetIndex(to),
                    paths: if *count == 1 {
                        PathCount::One
                    } else {
                        PathCount::Several
                    },
                })
                .collect();
            assert_eq!(*reached, expected, "graph {edges:?} from {from}");
        }
        let expected_pairs = reachable
            .iter()
            .flatten()
            .filter(|reached| reached.paths == PathCount::Several)
            .count();
        assert_eq!(pairs.len(), expected_pairs);
    }
}
