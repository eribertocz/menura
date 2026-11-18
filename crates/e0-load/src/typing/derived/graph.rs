//! The graph of the metrics that derived and context metrics read (E0G-5.1, DER-5.1.4,
//! CTX-5.1.5): its cycles and, when it has none, its topological order.

use std::collections::HashMap;

use crate::pointer::JsonPointer;
use crate::read::Location;
use crate::references::context::ContextMetric;
use crate::references::derived::DerivedMetric;

/// A metric that reads other metrics: a node of the graph with edges.
#[derive(Debug, Clone, Copy)]
pub(super) enum Dependent<'resolved> {
    /// A derived metric, with one edge per `AGG(…)` term in the order written (DER-5.1.4).
    Derived(&'resolved DerivedMetric),
    /// A context metric, with one edge to its base (CTX-5.1.5).
    Context(&'resolved ContextMetric),
}

impl<'resolved> Dependent<'resolved> {
    pub(super) fn pointer(self) -> &'resolved JsonPointer {
        match self {
            Self::Derived(metric) => &metric.pointer,
            Self::Context(metric) => &metric.pointer,
        }
    }

    pub(super) fn name(self) -> &'resolved str {
        match self {
            Self::Derived(metric) => &metric.name,
            Self::Context(metric) => &metric.name,
        }
    }

    /// Where the diagnostics about the metric are located.
    pub(super) fn diagnostic_place(self) -> (&'resolved JsonPointer, Location) {
        match self {
            Self::Derived(metric) => (&metric.diagnostic_pointer, metric.diagnostic_location),
            Self::Context(metric) => (&metric.diagnostic_pointer, metric.diagnostic_location),
        }
    }

    /// The metrics it reads, in the order of its edges.
    fn reads(self) -> Vec<&'resolved JsonPointer> {
        match self {
            Self::Derived(metric) => metric.inputs.iter().map(|input| &input.metric).collect(),
            Self::Context(metric) => vec![&metric.base],
        }
    }
}

/// What the depth-first search of the graph found.
pub(super) struct Graph<'resolved> {
    /// Every cycle, each once, starting and ending at its metric that comes first in document
    /// order (`a → b → a`; `a → a` for a metric that reads itself), in the order the search
    /// found them.
    pub(super) cycles: Vec<Vec<Dependent<'resolved>>>,
    /// Every dependent metric, each after the dependent metrics it reads; meaningful only
    /// without cycles.
    pub(super) order: Vec<Dependent<'resolved>>,
}

/// Where the search is with a metric.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Unvisited,
    OnPath,
    Done,
}

/// Searches the graph of `metrics`, which are in document order, depth first: from each metric
/// in document order and along its edges in order (DER-5.1.4). Base metrics read no metric,
/// so only the derived and context metrics have edges.
pub(super) fn search<'resolved>(metrics: &[Dependent<'resolved>]) -> Graph<'resolved> {
    let index: HashMap<&JsonPointer, usize> = metrics
        .iter()
        .enumerate()
        .map(|(position, metric)| (metric.pointer(), position))
        .collect();
    let edges: Vec<Vec<usize>> = metrics
        .iter()
        .map(|metric| {
            metric
                .reads()
                .into_iter()
                .filter_map(|read| index.get(read).copied())
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
    let by_index = |cycle: &Vec<usize>| cycle.iter().map(|node| metrics[*node]).collect();
    Graph {
        cycles: search.cycles.iter().map(by_index).collect(),
        order: search.order.iter().map(|node| metrics[*node]).collect(),
    }
}
/// The state of a depth-first search over the dependent metrics, by their index in document
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
