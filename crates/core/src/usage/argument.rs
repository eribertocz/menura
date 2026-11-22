//! The arguments of the commands, by neutral name (ARC-5.5 to ARC-5.9).

/// One argument of a command: a document, an options object, the audit-record destination or
/// the dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Argument {
    /// `model`: the model document.
    Model,
    /// `catalog_schema`: the catalog schema document.
    CatalogSchema,
    /// `catalog_state`: the catalog state document.
    CatalogState,
    /// `query`: the query document.
    Query,
    /// `security_context`: the security context document.
    SecurityContext,
    /// `load_options`: the load options object.
    LoadOptions,
    /// `compile_options`: the compile options object.
    CompileOptions,
    /// `audit_record`: where `compile` writes the audit record (CLI only, ARC-5.7).
    AuditRecord,
    /// `dialect`: the dialect of `integrity-checks` (ARC-5.8).
    Dialect,
    /// `old_model`: the old model document of `diff`.
    OldModel,
    /// `old_catalog_schema`: the catalog schema of the old model of `diff`.
    OldCatalogSchema,
    /// `new_model`: the new model document of `diff`.
    NewModel,
    /// `new_catalog_schema`: the catalog schema of the new model of `diff`.
    NewCatalogSchema,
}

impl Argument {
    /// Every argument.
    pub const ALL: [Self; 13] = [
        Self::Model,
        Self::CatalogSchema,
        Self::CatalogState,
        Self::Query,
        Self::SecurityContext,
        Self::LoadOptions,
        Self::CompileOptions,
        Self::AuditRecord,
        Self::Dialect,
        Self::OldModel,
        Self::OldCatalogSchema,
        Self::NewModel,
        Self::NewCatalogSchema,
    ];

    /// The neutral name, such as `catalog_schema`.
    ///
    /// ```
    /// use menura_core::Argument;
    ///
    /// assert_eq!(Argument::CatalogSchema.name(), "catalog_schema");
    /// assert_eq!(Argument::from_name("catalog_schema"), Some(Argument::CatalogSchema));
    /// ```
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Model => "model",
            Self::CatalogSchema => "catalog_schema",
            Self::CatalogState => "catalog_state",
            Self::Query => "query",
            Self::SecurityContext => "security_context",
            Self::LoadOptions => "load_options",
            Self::CompileOptions => "compile_options",
            Self::AuditRecord => "audit_record",
            Self::Dialect => "dialect",
            Self::OldModel => "old_model",
            Self::OldCatalogSchema => "old_catalog_schema",
            Self::NewModel => "new_model",
            Self::NewCatalogSchema => "new_catalog_schema",
        }
    }

    /// The argument whose neutral name is `name`.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|argument| argument.name() == name)
    }
}
