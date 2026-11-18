//! The graph of the metrics that derived metrics read (E0G-5.1, DER-5.1.4): its cycles and, when
//! it has none, its topological order.

use std::collections::HashMap;

use crate::references::derived::{DerivedMetric, DerivedMetrics};

/// What the depth-first search of the graph found.
pub(super) struct Graph<'derived> {
    /// Every cycle, each once, starting and ending at its metric that comes first in document
    /// order (`a → b → a`; `a → a` for a metric that reads itself), in the order the search
    /// found them.
    pub(super) cycles: Vec<Vec<&'derived DerivedMetric>>,
    /// Every derived metric, each after the derived metrics it reads; meaningful only without
    /// cycles.
    pub(super) order: Vec<&'derived DerivedMetric>,
}

/// Where the search is with a derived metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Unvisited,
    OnPath,
    Done,
}

/// Searches the graph of `derived` depth first, from each derived metric in document order and
/// along the `AGG(…)` terms in the order they are written (DER-5.1.4). Base metrics read no
/// metric, so only the derived metrics have edges.
pub(super) fn search(derived: &DerivedMetrics) -> Graph<'_> {
    let metrics = &derived.metrics;
    let index: HashMap<_, usize> = metrics
        .iter()
        .enumerate()
        .map(|(position, metric)| (&metric.pointer, position))
        .collect();
    let edges: Vec<Vec<usize>> = metrics
        .iter()
        .map(|metric| {
            metric
                .inputs
                .iter()
                .filter_map(|input| index.get(&input.metric).copied())
                .collect()
        })
        .collect();
    let mut search = Search {
        edges: &edges,
        states: vec![State::Unvisited; metrics.len()],
        path: Vec::new(),
        cycles: Vec::new(),
        order: Vec::new(),
    };
    for start in 0..metrics.len() {
        if search.states[start] == State::Unvisited {
            search.visit(start);
        }
    }
    let by_index = |cycle: &Vec<usize>| cycle.iter().map(|node| &metrics[*node]).collect();
    Graph {
        cycles: search.cycles.iter().map(by_index).collect(),
        order: search.order.iter().map(|node| &metrics[*node]).collect(),
    }
}

/// The state of a depth-first search over the derived metrics, by their index in document
/// order.
struct Search<'edges> {
    edges: &'edges [Vec<usize>],
    states: Vec<State>,
    path: Vec<usize>,
    cycles: Vec<Vec<usize>>,
    order: Vec<usize>,
}

impl Search<'_> {
    fn visit(&mut self, node: usize) {
        self.states[node] = State::OnPath;
        self.path.push(node);
        for &next in &self.edges[node] {
            match self.states[next] {
                State::Unvisited => self.visit(next),
                State::OnPath => self.record_cycle(next),
                State::Done => {}
            }
        }
        self.path.pop();
        self.states[node] = State::Done;
        self.order.push(node);
    }

    /// Records the cycle that closes at `start`, on the current path, unless it is known: it
    /// starts and ends at its metric that comes first in document order.
    fn record_cycle(&mut self, start: usize) {
        let Some(from) = self.path.iter().position(|node| *node == start) else {
            return;
        };
        let members = &self.path[from..];
        let first = members
            .iter()
            .enumerate()
            .min_by_key(|(_, node)| **node)
            .map_or(0, |(position, _)| position);
        let mut cycle: Vec<usize> = members[first..]
            .iter()
            .chain(&members[..first])
            .copied()
            .collect();
        cycle.push(members[first]);
        if !self.cycles.contains(&cycle) {
            self.cycles.push(cycle);
        }
    }
}
