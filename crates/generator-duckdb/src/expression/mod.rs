//! Writes one expression of a plan node as `DuckDB` SQL.

mod aggregate;
mod arithmetic;
mod attributes;
mod comparisons;
mod conditionals;
mod current;
mod dates;
mod functions;
mod numbers;
mod operators;
mod text;
mod time;

use menura_contracts::normalize_unquoted_identifier;
use menura_contracts::types::catalog::{CatalogSchemaDocument, Column, RelationSchema};
use menura_model::expression::{Expression, Identifier, Literal, Reference};
use menura_model::internal_model::InternalModel;
use menura_representations::logical_plan::OutputSchema;
use menura_representations::origin::Origin;
use menura_representations::time::QueryTime;
use menura_security::AdmittedSecurityContext;

use crate::error::{GenerateError, InvariantViolation};
use crate::identifiers::quote;
use crate::parameters::Parameters;
use crate::values::{literal_sql, literal_value};

/// What every expression of a statement may read: the catalog schema, the model, the security
/// context and the time settings of the query.
pub(crate) struct Environment<'input> {
    /// The admitted catalog schema, for the columns of each dataset `source`.
    pub(crate) catalog: &'input CatalogSchemaDocument,
    /// The internal model, for attribute declarations.
    pub(crate) model: &'input InternalModel,
    /// The admitted security context, for attribute values.
    pub(crate) security: &'input AdmittedSecurityContext,
    /// The query `time_zone` (E6-5.2) and its reference time (E0F-2.3).
    pub(crate) time: &'input QueryTime,
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
    /// A column of the catalog relation of the dataset `source`, in a field expression.
    Catalog(&'input RelationSchema),
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
            _ => self.write_call(expression),
        }
    }

    /// The SQL of a conversion, a function call or a value of the reference time.
    fn write_call(&mut self, expression: &Expression) -> Result<String, GenerateError> {
        match expression {
            Expression::Cast { operand, target } => self.cast(operand, *target),
            Expression::DateTrunc { unit, operand } => self.date_trunc(*unit, operand),
            Expression::Like {
                operand,
                pattern,
                case,
                negated,
            } => self.like(operand, pattern, *case, *negated),
            Expression::Extract { part, operand } => self.extract(expression, *part, operand),
            Expression::DateAdd {
                unit,
                amount,
                operand,
            } => self.date_add(expression, *unit, amount, operand),
            Expression::DateDiff { unit, start, end } => {
                self.date_diff(expression, *unit, start, end)
            }
            Expression::Function {
                function,
                arguments,
            } => self.function(expression, *function, arguments),
            Expression::CurrentDate => self.current_date(),
            Expression::CurrentTime | Expression::CurrentTimestamp => {
                self.current_instant(expression)
            }
            Expression::Aggregate { call, filter } => self.aggregate(call, filter.as_deref()),
            // `MetricAggregate` is outside the 0.3.0 slice (VS-1).
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
                | Expression::Like { .. }
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

    /// An input column by its name, or a catalog column as the catalog spells it (PLN-2.2).
    fn reference(&self, reference: &Reference) -> Result<String, GenerateError> {
        if reference.qualifier.is_some() {
            return Err(self.error(InvariantViolation::QualifiedReference));
        }
        let name = reference.name.text();
        let known = match self.columns {
            Columns::Input(schema) => schema
                .columns()
                .iter()
                .any(|column| column.name.as_str() == name)
                .then(|| name.to_owned()),
            Columns::Catalog(relation) => {
                catalog_column(relation, &reference.name).map(|column| column.name.clone())
            }
        };
        known
            .map(|name| quote(&name))
            .ok_or_else(|| self.error(InvariantViolation::UnknownColumn))
    }

    /// The error `violation` in this node.
    fn error(&self, violation: InvariantViolation) -> GenerateError {
        GenerateError::new(violation, self.origin)
    }
}

/// The column of `relation` that `name` matches: the column whose name, normalized as E0D-1.1,
/// equals the normalized text of `name`. E0 has already resolved the reference (E0D-1.5) and
/// stores it as the delimited identifier of its normalized text, and CAT-8.2 makes the
/// normalized match unique.
pub(crate) fn catalog_column<'relation>(
    relation: &'relation RelationSchema,
    name: &Identifier,
) -> Option<&'relation Column> {
    catalog_column_named(relation, name.text())
}

/// The column of `relation` whose name, normalized as E0D-1.1, equals `name` normalized the
/// same way (CAT-8.2 makes it unique).
pub(crate) fn catalog_column_named<'relation>(
    relation: &'relation RelationSchema,
    name: &str,
) -> Option<&'relation Column> {
    let wanted = normalize_unquoted_identifier(name);
    relation
        .columns
        .iter()
        .find(|column| normalize_unquoted_identifier(&column.name) == wanted)
}
