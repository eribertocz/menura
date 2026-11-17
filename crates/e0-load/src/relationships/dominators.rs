//! The dominator tree of a directed graph from one source (Lengauer and Tarjan, with path
//! compression): a vertex `d` dominates `v` when every path from the source to `v` passes
//! through `d`.
//!
//! Vertices are numbered `0..n`. Inside, the vertices reached from the source are renumbered in
//! depth-first preorder, so the source is 0 and every immediate dominator has a lower number
//! than the vertices it dominates.

/// The dominator tree of the vertices reached from one source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DominatorTree {
    /// The vertices reached from the source, in depth-first preorder; the source first.
    preorder: Vec<usize>,
    /// The preorder number of each vertex; `None` when it is not reached.
    number: Vec<Option<usize>>,
    /// The preorder number of the immediate dominator, by preorder number; the source has
    /// itself.
    immediate: Vec<usize>,
    /// The interval of each preorder number in a depth-first walk of the dominator tree, so that
    /// dominance is a containment test.
    interval: Vec<(usize, usize)>,
}

impl DominatorTree {
    /// The dominator tree from `source` of the graph whose edges leave each vertex towards
    /// `successors[vertex]` and arrive at each vertex from `predecessors[vertex]`.
    ///
    /// Runs in O(E log V) time for the E edges and V vertices reached.
    pub(crate) fn new(
        source: usize,
        successors: &[Vec<usize>],
        predecessors: &[Vec<usize>],
    ) -> Self {
        let walk = depth_first(source, successors);
        let immediate = immediate_dominators(&walk, predecessors);
        let interval = intervals(&immediate);
        Self {
            preorder: walk.preorder,
            number: walk.number,
            immediate,
            interval,
        }
    }

    /// The vertices reached from the source, in depth-first preorder; the source first. Each
    /// vertex comes after its immediate dominator.
    pub(crate) fn reached(&self) -> &[usize] {
        &self.preorder
    }

    /// Whether `vertex` is reached from the source.
    pub(crate) fn reaches(&self, vertex: usize) -> bool {
        self.number_of(vertex).is_some()
    }

    /// The immediate dominator of `vertex`; `None` for the source and for vertices not reached.
    pub(crate) fn immediate_dominator(&self, vertex: usize) -> Option<usize> {
        let number = self.number_of(vertex).filter(|number| *number != 0)?;
        self.preorder.get(*self.immediate.get(number)?).copied()
    }

    /// Whether `dominator` dominates `vertex`; every vertex dominates itself. False when either
    /// is not reached.
    pub(crate) fn dominates(&self, dominator: usize, vertex: usize) -> bool {
        let (Some(outer), Some(inner)) = (self.number_of(dominator), self.number_of(vertex)) else {
            return false;
        };
        match (self.interval.get(outer), self.interval.get(inner)) {
            (Some(outer), Some(inner)) => outer.0 <= inner.0 && inner.1 <= outer.1,
            _ => false,
        }
    }

    fn number_of(&self, vertex: usize) -> Option<usize> {
        self.number.get(vertex).copied().flatten()
    }
}

/// A depth-first walk from the source.
struct Walk {
    /// The vertices reached, in preorder.
    preorder: Vec<usize>,
    /// The preorder number of each vertex, if reached.
    number: Vec<Option<usize>>,
    /// The preorder number of the parent in the depth-first tree, by preorder number; the source
    /// has itself.
    parent: Vec<usize>,
}

fn depth_first(source: usize, successors: &[Vec<usize>]) -> Walk {
    let mut walk = Walk {
        preorder: vec![source],
        number: vec![None; successors.len()],
        parent: vec![0],
    };
    if let Some(slot) = walk.number.get_mut(source) {
        *slot = Some(0);
    }
    let mut stack = vec![(source, 0_usize)];
    while let Some((vertex, next)) = stack.last_mut() {
        let vertex = *vertex;
        let Some(&successor) = successors.get(vertex).and_then(|list| list.get(*next)) else {
            stack.pop();
            continue;
        };
        *next += 1;
        let parent = walk.number.get(vertex).copied().flatten().unwrap_or(0);
        if let Some(slot @ None) = walk.number.get_mut(successor) {
            *slot = Some(walk.preorder.len());
            walk.preorder.push(successor);
            walk.parent.push(parent);
            stack.push((successor, 0));
        }
    }
    walk
}

/// The state of the Lengauer-Tarjan algorithm, by preorder number.
struct Forest {
    /// The semidominator of each vertex.
    semi: Vec<usize>,
    /// The ancestor of each vertex in the forest of processed vertices.
    ancestor: Vec<Option<usize>>,
    /// The vertex with the lowest semidominator on the forest path above each vertex.
    best: Vec<usize>,
}

impl Forest {
    fn new(size: usize) -> Self {
        Self {
            semi: (0..size).collect(),
            ancestor: vec![None; size],
            best: (0..size).collect(),
        }
    }

    fn semi(&self, vertex: usize) -> usize {
        self.semi.get(vertex).copied().unwrap_or(vertex)
    }

    fn ancestor(&self, vertex: usize) -> Option<usize> {
        self.ancestor.get(vertex).copied().flatten()
    }

    fn best(&self, vertex: usize) -> usize {
        self.best.get(vertex).copied().unwrap_or(vertex)
    }

    fn link(&mut self, parent: usize, vertex: usize) {
        if let Some(slot) = self.ancestor.get_mut(vertex) {
            *slot = Some(parent);
        }
    }

    /// The vertex with the lowest semidominator on the forest path above `vertex`, compressing
    /// the path on the way.
    fn evaluate(&mut self, vertex: usize) -> usize {
        let mut path = Vec::new();
        let mut current = vertex;
        while let Some(ancestor) = self.ancestor(current) {
            if self.ancestor(ancestor).is_none() {
                break;
            }
            path.push(current);
            current = ancestor;
        }
        for &step in path.iter().rev() {
            let Some(ancestor) = self.ancestor(step) else {
                continue;
            };
            let candidate = self.best(ancestor);
            if self.semi(candidate) < self.semi(self.best(step))
                && let Some(slot) = self.best.get_mut(step)
            {
                *slot = candidate;
            }
            let above = self.ancestor(ancestor);
            if let Some(slot) = self.ancestor.get_mut(step) {
                *slot = above;
            }
        }
        self.best(vertex)
    }
}

/// The immediate dominator of each preorder number.
fn immediate_dominators(walk: &Walk, predecessors: &[Vec<usize>]) -> Vec<usize> {
    let size = walk.preorder.len();
    let mut forest = Forest::new(size);
    let mut immediate = vec![0; size];
    let mut same_dominator: Vec<Option<usize>> = vec![None; size];
    let mut bucket: Vec<Vec<usize>> = vec![Vec::new(); size];
    for (number, &vertex) in walk.preorder.iter().enumerate().skip(1).rev() {
        let parent = walk.parent.get(number).copied().unwrap_or(0);
        let semi = semidominator(
            &mut forest,
            walk,
            predecessors.get(vertex),
            (number, parent),
        );
        if let Some(slot) = forest.semi.get_mut(number) {
            *slot = semi;
        }
        if let Some(waiting) = bucket.get_mut(semi) {
            waiting.push(number);
        }
        forest.link(parent, number);
        for waiting in bucket
            .get_mut(parent)
            .map(std::mem::take)
            .unwrap_or_default()
        {
            let best = forest.evaluate(waiting);
            if forest.semi(best) == forest.semi(waiting) {
                if let Some(slot) = immediate.get_mut(waiting) {
                    *slot = parent;
                }
            } else if let Some(slot) = same_dominator.get_mut(waiting) {
                *slot = Some(best);
            }
        }
    }
    for number in 1..size {
        if let Some(Some(same)) = same_dominator.get(number).copied() {
            let dominator = immediate.get(same).copied().unwrap_or(0);
            if let Some(slot) = immediate.get_mut(number) {
                *slot = dominator;
            }
        }
    }
    immediate
}

/// The semidominator of the vertex numbered `number`, whose depth-first parent is `parent` and
/// whose predecessors are `predecessors`: the lowest number from which a path reaches it
/// through vertices numbered above it only.
fn semidominator(
    forest: &mut Forest,
    walk: &Walk,
    predecessors: Option<&Vec<usize>>,
    (number, parent): (usize, usize),
) -> usize {
    let numbered = predecessors
        .into_iter()
        .flatten()
        .filter_map(|predecessor| walk.number.get(*predecessor).copied().flatten());
    let mut semi = parent;
    for predecessor in numbered {
        let candidate = if predecessor <= number {
            predecessor
        } else {
            let best = forest.evaluate(predecessor);
            forest.semi(best)
        };
        semi = semi.min(candidate);
    }
    semi
}

/// The interval of each preorder number in a depth-first walk of the dominator tree given by
/// `immediate`: `d` dominates `v` exactly when the interval of `d` contains that of `v`.
fn intervals(immediate: &[usize]) -> Vec<(usize, usize)> {
    let size = immediate.len();
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); size];
    for (number, &dominator) in immediate.iter().enumerate().skip(1) {
        if let Some(list) = children.get_mut(dominator) {
            list.push(number);
        }
    }
    let mut interval = vec![(0, 0); size];
    let mut clock = 0;
    let mut stack = vec![(0_usize, 0_usize)];
    if let Some(root) = interval.get_mut(0) {
        root.0 = clock;
    }
    while let Some((vertex, next)) = stack.last_mut() {
        let vertex = *vertex;
        clock += 1;
        if let Some(&child) = children.get(vertex).and_then(|list| list.get(*next)) {
            *next += 1;
            if let Some(slot) = interval.get_mut(child) {
                slot.0 = clock;
            }
            stack.push((child, 0));
        } else {
            if let Some(slot) = interval.get_mut(vertex) {
                slot.1 = clock;
            }
            stack.pop();
        }
    }
    interval
}
