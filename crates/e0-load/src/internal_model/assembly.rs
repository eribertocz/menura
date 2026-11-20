//! What every part of the assembly reads: the substage outputs, the model outline, the
//! compiled variants, the types and the usability of each element.

use std::collections::HashMap;

use menura_model::expression::Expression;
use menura_model::internal_model::{TypedValue, ValueType};

use super::ModelInput;
use super::index::ModelIndex;
use super::trees::compiled;
use super::usability::Usability;
use crate::expressions::ExpressionPlace;
use crate::outline::ModelOutline;
use crate::pointer::JsonPointer;
use crate::read::Node;
use crate::structure::CompiledVariant;
use crate::typing::TypedElement;

/// The shared, read-only state of one assembly.
pub(super) struct Assembly<'model, 'document> {
    /// The model document.
    pub(super) document: &'document Node,
    /// The substage outputs.
    pub(super) input: &'model ModelInput<'model, 'document>,
    /// The elements of the model, by name.
    pub(super) index: ModelIndex<'model, 'document>,
    /// The usability of every element, after propagation.
    pub(super) usability: Usability,
    variants: HashMap<&'model JsonPointer, CompiledVariant<'document>>,
    types: HashMap<&'model JsonPointer, &'model TypedElement>,
}

impl<'model, 'document> Assembly<'model, 'document> {
    /// The assembly state of `model`.
    pub(super) fn new(
        document: &'document Node,
        input: &'model ModelInput<'model, 'document>,
        model: &'model ModelOutline<'document>,
        usability: Usability,
    ) -> Self {
        Self {
            document,
            input,
            index: ModelIndex::new(model),
            usability,
            variants: input
                .structure
                .expressions
                .iter()
                .map(|expression| (&expression.pointer, expression.variant))
                .collect(),
            types: input
                .typing
                .elements
                .iter()
                .chain(&input.derived_typing.elements)
                .map(|element| (&element.pointer, element))
                .collect(),
        }
    }

    /// The normalized tree of the element at `pointer`, when its compiled variant parses.
    pub(super) fn tree(&self, pointer: &JsonPointer, place: ExpressionPlace) -> Option<Expression> {
        self.variants
            .get(pointer)
            .and_then(|variant| compiled(*variant, place))
    }

    /// The type E0.f found for the element at `pointer`.
    pub(super) fn typed(&self, pointer: &JsonPointer) -> Option<TypedValue> {
        let element = self.types.get(pointer)?;
        Some(TypedValue {
            value_type: element.typed.logical().map_or(ValueType::Null, |logical| {
                ValueType::Logical(logical.clone())
            }),
            nullable: element.typed.nullable,
        })
    }
}
