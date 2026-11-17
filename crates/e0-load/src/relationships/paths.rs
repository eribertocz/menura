//! Reachability and the number of paths between datasets (E0E-2.4, E0E-4, E0E-6), without
//! enumerating paths.
//!
//! A path is a sequence of edges that never repeats a dataset (E0E-2.4). From a source `s`,
//! call an edge `x → v` *usable* when `v` does not dominate `x`: some path reaches `x` without
//! `v`, so the edge can end a path to `v`. Then:
//!
//! - two usable edges into `v` give two paths to `v`, which differ in their last edge;
//! - with exactly one usable edge `x → v`, every path to `v` ends with it, `x` is the immediate
//!   dominator of `v`, and the paths to `v` are the paths to `x` followed by that edge.
//!
//! So `v` has more than one path exactly when some dataset on its dominator chain, other than
//! `s`, has two usable edges into it. With the dominator tree, each source costs O(E log V).

use super::dominators::DominatorTree;
use super::output::{DatasetIndex, DatasetPair, PathCount, Reachable, RelationshipGraph};

/// What every dataset of `graph` reaches, by [`DatasetIndex`], and the pairs with more than one
/// path, by source then target in graph order.
pub(crate) fn reachability(graph: &RelationshipGraph) -> (Vec<Vec<Reachable>>, Vec<DatasetPair>) {
    let size = graph.datasets.len();
    let mut successors = vec![Vec::new(); size];
    let mut predecessors = vec![Vec::new(); size];
    for edge in &graph.edges {
        if let Some(list) = successors.get_mut(edge.from.0) {
            list.push(edge.to.0);
        }
        if let Some(list) = predecessors.get_mut(edge.to.0) {
            list.push(edge.from.0);
        }
    }
    let mut reachable = Vec::with_capacity(size);
    let mut several_paths = Vec::new();
    for source in 0..size {
        let from_source = from(source, &successors, &predecessors);
        several_paths.extend(
            from_source
                .iter()
                .filter(|reached| reached.paths == PathCount::Several)
                .map(|reached| DatasetPair {
                    from: DatasetIndex(source),
                    to: reached.dataset,
                }),
        );
        reachable.push(from_source);
    }
    (reachable, several_paths)
}

/// The datasets reached from `source`, in graph order, with their path count.
fn from(source: usize, successors: &[Vec<usize>], predecessors: &[Vec<usize>]) -> Vec<Reachable> {
    let tree = DominatorTree::new(source, successors, predecessors);
    let mut several = vec![false; successors.len()];
    for &vertex in tree.reached().iter().skip(1) {
        let usable_edges = predecessors
            .get(vertex)
            .into_iter()
            .flatten()
            .filter(|&&predecessor| {
                tree.reaches(predecessor) && !tree.dominates(vertex, predecessor)
            })
            .take(2)
            .count();
        let through_dominator = tree
            .immediate_dominator(vertex)
            .and_then(|dominator| several.get(dominator).copied())
            .unwrap_or(false);
        if let Some(slot) = several.get_mut(vertex) {
            *slot = usable_edges >= 2 || through_dominator;
        }
    }
    let mut reached: Vec<Reachable> = tree
        .reached()
        .iter()
        .skip(1)
        .map(|&vertex| Reachable {
            dataset: DatasetIndex(vertex),
            paths: if several.get(vertex).copied().unwrap_or(false) {
                PathCount::Several
            } else {
                PathCount::One
            },
        })
        .collect();
    reached.sort_by_key(|reached| reached.dataset);
    reached
}
