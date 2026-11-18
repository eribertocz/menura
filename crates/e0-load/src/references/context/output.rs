//! The context metrics of a model and their bases, as E0.d resolves them, as data types only.

use crate::extensions::context::Modifier;
use crate::pointer::JsonPointer;
use crate::read::Location;

/// The context metrics of a model whose base and removed fields resolve.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ContextMetrics {
    /// Every context metric, in document order.
    pub metrics: Vec<ContextMetric>,
}

impl ContextMetrics {
    /// The context metric at `pointer`, when it is one.
    #[must_use]
    pub fn metric(&self, pointer: &JsonPointer) -> Option<&ContextMetric> {
        self.metrics
            .iter()
            .find(|metric| metric.pointer == *pointer)
    }
}

/// One context metric (CTX-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextMetric {
    /// The JSON Pointer of the metric.
    pub pointer: JsonPointer,
    /// Its name as written, for messages.
    pub name: String,
    /// The JSON Pointer of its base: a model metric that is not a context metric, which may be
    /// derived (CTX-3.7, CTX-5.1.3).
    pub base: JsonPointer,
    /// Its modifier, as E0.c read it.
    pub modifier: Modifier,
    /// Where the diagnostics about the metric are located: the `data` member of its entry of
    /// ours (E0C-2.6).
    pub diagnostic_pointer: JsonPointer,
    /// Where that node starts in the document text.
    pub diagnostic_location: Location,
}
