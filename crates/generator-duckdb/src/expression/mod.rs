//! Writes one expression of a plan node as `DuckDB` SQL.

mod aggregate;
mod arithmetic;
mod attributes;
mod comparisons;
mod operators;
mod time;

use menura_model::expression::{Expression, Literal, Reference};
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::OutputSchema;
use menura_representations::origin::Origin;
use menura_representations::time::TimeZoneName;
use menura_security::AdmittedSecurityContext;

use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::parameters::Parameters;
use crate::values::{literal_sql, literal_value};

/// What every expression of a statement may read: the model, the security context and the
/// query time zone.
pub(crate) struct Environment<'input> {
    /// The internal model, for attribute declarations.
    pub(crate) model: &'input InternalModel,
    /// The admitted security context, for attribute values.
    pub(crate) security: &'input AdmittedSecurityContext,
    /// The query `time_zone` (E6-5.2).
    pub(crate) time_zone: &'input TimeZoneName,
}

/// Whether the literals of a node are query values or model constants (E6-4.2, E6-4.11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Literals {
    /// A node whose origin is the query: every literal is a bound parameter, except `NULL`.
    Bound,
    /// A node whose origin is the model: literals are written in the SQL.
    Written,
}

impl Literals {
    /// The literal rule of a node that comes from `origin`.
    pub(crate) fn of(origin: &Origin) -> Self {
        match origin {
            Origin::Query(_) => Self::Bound,
            Origin::Model(_) => Self::Written,
        }
    }
}

/// What an unqualified reference names.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Columns<'input> {
    /// A column of the input node.
    Input(&'input OutputSchema),
    /// A catalog column of the dataset `source`, in a field expression.
    Catalog,
}

/// Writes the expressions of one plan node.
pub(crate) struct ExpressionWriter<'input, 'statement> {
    environment: &'input Environment<'input>,
    literals: Literals,
    columns: Columns<'input>,
    origin: &'input Origin,
    parameters: &'statement mut Parameters,
}

impl<'input, 'statement> ExpressionWriter<'input, 'statement> {
    /// A writer for the node from `origin`, whose references name `columns`.
    pub(crate) fn new(
        environment: &'input Environment<'input>,
        columns: Columns<'input>,
        origin: &'input Origin,
        parameters: &'statement mut Parameters,
    ) -> Self {
        Self {
            environment,
            literals: Literals::of(origin),
            columns,
            origin,
            parameters,
        }
    }

    /// The SQL of `expression`, binding its parameters in the order they appear in it.
    pub(crate) fn write(&mut self, expression: &Expression) -> Result<String, GenerateError> {
        match expression {
            Expression::Literal(literal) => self.literal(literal),
            Expression::Reference(reference) => self.reference(reference),
            Expression::UserAttribute(name) => self.scalar_attribute(name),
            Expression::Unary { operator, operand } => self.unary(expression, *operator, operand),
            Expression::Binary {
                operator,
                left,
                right,
            } => self.binary(expression, *operator, left, right),
            Expression::IsNull { operand, negated } => self.is_null(operand, *negated),
            Expression::IsDistinctFrom {
                left,
                right,
                negated,
            } => self.is_distinct_from(left, right, *negated),
            Expression::InList {
                operand,
                list,
                negated,
            } => self.in_list(operand, list, *negated),
            Expression::Between {
                operand,
                low,
                high,
                negated,
            } => self.between(operand, low, high, *negated),
            Expression::Case {
                operand,
                branches,
                otherwise,
            } => self.case(operand.as_deref(), branches, otherwise.as_deref()),
            Expression::Cast { operand, target } => self.cast(operand, *target),
            Expression::DateTrunc { unit, operand } => self.date_trunc(*unit, operand),
            Expression::Aggregate { call, filter } => self.aggregate(call, filter.as_deref()),
            _ => Err(self.error(InvariantViolation::ExpressionOutsideSlice)),
        }
    }

    /// The SQL of `expression` as an operand: between parentheses when it is an operator.
    fn operand(&mut self, expression: &Expression) -> Result<String, GenerateError> {
        let text = self.write(expression)?;
        let compound = matches!(
            expression,
            Expression::Unary { .. }
                | Expression::Binary { .. }
                | Expression::IsNull { .. }
                | Expression::IsDistinctFrom { .. }
                | Expression::InList { .. }
                | Expression::Between { .. }
        );
        Ok(if compound { format!("({text})") } else { text })
    }

    fn literal(&mut self, literal: &Literal) -> Result<String, GenerateError> {
        if self.literals == Literals::Written || *literal == Literal::Null {
            return Ok(literal_sql(literal));
        }
        let logical_type = self.literal_type(literal)?;
        let value =
            literal_value(literal).ok_or_else(|| self.error(InvariantViolation::NonFiniteFloat))?;
        Ok(self.parameters.bind(logical_type, value))
    }

    fn reference(&self, reference: &Reference) -> Result<String, GenerateError> {
        if reference.qualifier.is_some() {
            return Err(self.error(InvariantViolation::QualifiedReference));
        }
        let name = reference.name.text();
        if let Columns::Input(schema) = self.columns {
            let known = schema
                .columns()
                .iter()
                .any(|column| column.name.as_str() == name);
            if !known {
                return Err(self.error(InvariantViolation::UnknownColumn));
            }
        }
        Ok(quote(name))
    }

    /// The error `violation` in this node.
    fn error(&self, violation: InvariantViolation) -> GenerateError {
        GenerateError::new(violation, self.origin)
    }
}
