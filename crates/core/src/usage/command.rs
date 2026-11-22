//! The commands and the arguments each one takes (ARC-5.5 to ARC-5.9).

use super::argument::Argument;
use super::error::{UsageError, UsageReason};

/// One command of the CLI, and its MCP tool (ARC-6.7).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandName {
    /// `validate` (ARC-5.5).
    Validate,
    /// `compile` (ARC-5.7).
    Compile,
    /// `explain` (ARC-5.7).
    Explain,
    /// `describe` (ARC-5.8).
    Describe,
    /// `integrity_checks`, written `integrity-checks` on the command line (ARC-5.8).
    IntegrityChecks,
    /// `dependencies` (ARC-5.6).
    Dependencies,
    /// `migrate` (ARC-5.9).
    Migrate,
    /// `diff` (ARC-5.6).
    Diff,
}

impl CommandName {
    /// Every command, in the order of the delivery-forms table (section 3.6).
    pub const ALL: [Self; 8] = [
        Self::Validate,
        Self::Compile,
        Self::Explain,
        Self::Describe,
        Self::IntegrityChecks,
        Self::Dependencies,
        Self::Migrate,
        Self::Diff,
    ];

    /// The neutral name, such as `integrity_checks`.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Validate => "validate",
            Self::Compile => "compile",
            Self::Explain => "explain",
            Self::Describe => "describe",
            Self::IntegrityChecks => "integrity_checks",
            Self::Dependencies => "dependencies",
            Self::Migrate => "migrate",
            Self::Diff => "diff",
        }
    }

    /// The command whose neutral name is `name`.
    ///
    /// # Errors
    ///
    /// [`UsageReason::UnknownCommand`] when no command has that name.
    ///
    /// ```
    /// use menura_core::CommandName;
    ///
    /// assert_eq!(CommandName::from_name("integrity_checks"), Ok(CommandName::IntegrityChecks));
    /// assert!(CommandName::from_name("run").is_err());
    /// ```
    pub fn from_name(name: &str) -> Result<Self, UsageError> {
        Self::ALL
            .into_iter()
            .find(|command| command.name() == name)
            .ok_or_else(|| UsageError::new(None, UsageReason::UnknownCommand(name.to_owned())))
    }

    /// The arguments the command requires, in the order a missing one is reported.
    #[must_use]
    pub fn required_arguments(self) -> &'static [Argument] {
        use Argument::{
            CatalogSchema, Model, NewCatalogSchema, NewModel, OldCatalogSchema, OldModel, Query,
            SecurityContext,
        };
        match self {
            Self::Validate | Self::Dependencies | Self::IntegrityChecks => &[Model, CatalogSchema],
            Self::Compile | Self::Explain => &[Model, CatalogSchema, Query, SecurityContext],
            Self::Describe => &[Model, CatalogSchema, SecurityContext],
            Self::Migrate => &[Model],
            Self::Diff => &[OldModel, OldCatalogSchema, NewModel, NewCatalogSchema],
        }
    }

    /// The optional arguments of the command.
    #[must_use]
    pub fn optional_arguments(self) -> &'static [Argument] {
        use Argument::{AuditRecord, CatalogState, CompileOptions, Dialect, LoadOptions};
        match self {
            Self::Validate | Self::Dependencies | Self::Migrate | Self::Diff => &[LoadOptions],
            Self::Compile => &[CatalogState, LoadOptions, CompileOptions, AuditRecord],
            Self::Explain => &[CatalogState, LoadOptions, CompileOptions],
            Self::Describe => &[CatalogState, LoadOptions],
            Self::IntegrityChecks => &[LoadOptions, Dialect],
        }
    }

    /// The argument named `name` when the command takes it.
    ///
    /// # Errors
    ///
    /// [`UsageReason::UnknownArgument`] when the command takes no argument of that name: `explain`
    /// refuses `audit_record` this way (ARC-5.7).
    pub fn argument(self, name: &str) -> Result<Argument, UsageError> {
        Argument::from_name(name)
            .filter(|argument| self.takes(*argument))
            .ok_or_else(|| UsageError::new(None, UsageReason::UnknownArgument(name.to_owned())))
    }

    /// Checks that `given`, the arguments a call names, holds every required argument of the
    /// command.
    ///
    /// # Errors
    ///
    /// [`UsageReason::Missing`] for the first required argument `given` lacks.
    pub fn check_required(self, given: &[Argument]) -> Result<(), UsageError> {
        match self
            .required_arguments()
            .iter()
            .find(|argument| !given.contains(argument))
        {
            Some(missing) => Err(UsageError::new(Some(*missing), UsageReason::Missing)),
            None => Ok(()),
        }
    }

    fn takes(self, argument: Argument) -> bool {
        self.required_arguments().contains(&argument)
            || self.optional_arguments().contains(&argument)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arc_5_7_explain_refuses_audit_record_and_compile_takes_it() {
        assert_eq!(
            CommandName::Explain.argument("audit_record"),
            Err(UsageError::new(
                None,
                UsageReason::UnknownArgument("audit_record".to_owned())
            ))
        );
        assert_eq!(
            CommandName::Compile.argument("audit_record"),
            Ok(Argument::AuditRecord)
        );
    }

    #[test]
    fn arc_5_9_migrate_takes_no_catalog_argument() {
        assert!(CommandName::Migrate.argument("catalog_schema").is_err());
        assert_eq!(
            CommandName::Migrate.check_required(&[]),
            Err(UsageError::new(Some(Argument::Model), UsageReason::Missing))
        );
    }

    #[test]
    fn arc_5_6_the_first_missing_argument_is_reported() {
        assert_eq!(
            CommandName::Diff.check_required(&[Argument::OldModel, Argument::NewModel]),
            Err(UsageError::new(
                Some(Argument::OldCatalogSchema),
                UsageReason::Missing
            ))
        );
    }
}
