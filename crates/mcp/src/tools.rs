//! The tools of the server: one per CLI command, with its name, description and input schema
//! (ARC-6.7, ARC-6.9).

use menura_core::{Argument, CommandName};
use serde_json::{Map, Value, json};

/// What a member of a tool's input holds: a document as text, an options object, or a name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberKind {
    /// A document of type `string`, holding the file's text.
    Document,
    /// An options object of type `object`.
    Options,
    /// The dialect name, of type `string`.
    Name,
}

/// The kind of the member of `argument`.
pub(crate) fn member_kind(argument: Argument) -> MemberKind {
    match argument {
        Argument::LoadOptions | Argument::CompileOptions => MemberKind::Options,
        Argument::Dialect => MemberKind::Name,
        _ => MemberKind::Document,
    }
}

/// Whether the tool of `command` has a member for `argument`. `compile` returns its audit
/// record and has no `audit_record` member (ARC-6.7, ARC-6.8).
pub(crate) fn has_member(command: CommandName, argument: Argument) -> bool {
    argument != Argument::AuditRecord
        && (command.required_arguments().contains(&argument)
            || command.optional_arguments().contains(&argument))
}

/// The description of the tool of `command`, as the crate documentation lists it.
pub(crate) fn tool_description(command: CommandName) -> &'static str {
    match command {
        CommandName::Validate => {
            "Validates an Apache Ossie model against a catalog schema and returns the validation report."
        }
        CommandName::Compile => {
            "Compiles a query and returns the compiled plan, or the compile error; with the compile option `audit: true`, the audit record follows."
        }
        CommandName::Explain => {
            "Returns the explanation of a query: the compiled plan without SQL or parameters, or the compile error."
        }
        CommandName::Describe => {
            "Returns the model description a security context sees, or the compile error."
        }
        CommandName::IntegrityChecks => {
            "Returns the integrity-check document of a dialect (`duckdb`, the default), or the compile error."
        }
        CommandName::Dependencies => {
            "Returns the model-dependency document of a model, or the compile error."
        }
        CommandName::Migrate => {
            "Rewrites the deprecated forms of a model and returns the migration report, or the compile error; it takes no catalog."
        }
        CommandName::Diff => {
            "Loads two models with the same load options, old first, and returns the diff of their dependencies, or the compile error."
        }
    }
}

/// The description of the member of `argument`.
pub(crate) fn member_description(argument: Argument) -> &'static str {
    match argument {
        Argument::Model => "The model document (YAML or JSON) as text.",
        Argument::CatalogSchema => "The catalog schema document (JSON) as text.",
        Argument::CatalogState => "The catalog state document (JSON) as text.",
        Argument::Query => "The query document (JSON) as text.",
        Argument::SecurityContext => "The security context document (JSON) as text.",
        Argument::LoadOptions => "The load options; an omitted option takes its default.",
        Argument::CompileOptions => "The compile options; an omitted option takes its default.",
        Argument::AuditRecord => "",
        Argument::Dialect => "The dialect, `duckdb` (the default).",
        Argument::OldModel => "The old model document as text.",
        Argument::OldCatalogSchema => "The catalog schema of the old model as text.",
        Argument::NewModel => "The new model document as text.",
        Argument::NewCatalogSchema => "The catalog schema of the new model as text.",
    }
}

/// The input schema of the tool of `command`: an object with `additionalProperties: false`
/// whose members are the command's arguments (ARC-6.7).
pub(crate) fn input_schema(command: CommandName) -> Value {
    let mut properties = Map::new();
    let members = command
        .required_arguments()
        .iter()
        .chain(command.optional_arguments())
        .filter(|argument| has_member(command, **argument));
    for argument in members {
        let schema_type = match member_kind(*argument) {
            MemberKind::Options => "object",
            MemberKind::Document | MemberKind::Name => "string",
        };
        properties.insert(
            argument.name().to_owned(),
            json!({"type": schema_type, "description": member_description(*argument)}),
        );
    }
    let required: Vec<&str> = command
        .required_arguments()
        .iter()
        .map(|argument| argument.name())
        .collect();
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false,
    })
}

/// The result of `tools/list`: every tool, in the order of section 3.6 (ARC-6.7).
pub(crate) fn tool_list() -> Value {
    let tools: Vec<Value> = CommandName::ALL
        .into_iter()
        .map(|command| {
            json!({
                "name": command.name(),
                "description": tool_description(command),
                "inputSchema": input_schema(command),
            })
        })
        .collect();
    json!({ "tools": tools })
}
