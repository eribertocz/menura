//! The arguments of a tool call, checked as the CLI checks its command line (ARC-6.7): the
//! tool must exist, every member must be one the tool takes, of its type, and every required
//! member must be present. The errors are the typed usage errors of `core`.

use std::collections::BTreeMap;

use menura_core::{Argument, CommandName, UsageError, UsageReason};
use serde_json::Value;

use crate::tools::{MemberKind, has_member, member_kind};

/// A tool call whose members passed the syntax checks: the command and the value of each
/// member it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolCall {
    /// The command of the tool.
    pub(crate) command: CommandName,
    /// The value of each member, by argument.
    pub(crate) members: BTreeMap<Argument, MemberValue>,
}

/// The value of one member.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum MemberValue {
    /// A document or a name, as text.
    Text(String),
    /// An options value as JSON bytes, read by `core` as the CLI reads an options file.
    Options(Vec<u8>),
}

impl ToolCall {
    /// The text of the document or name of `argument`, when given.
    pub(crate) fn text(&self, argument: Argument) -> Option<&str> {
        match self.members.get(&argument) {
            Some(MemberValue::Text(text)) => Some(text),
            _ => None,
        }
    }

    /// The bytes of the document of `argument`, when given.
    pub(crate) fn bytes(&self, argument: Argument) -> Option<&[u8]> {
        self.text(argument).map(str::as_bytes)
    }

    /// The bytes of the document of the required `argument`.
    pub(crate) fn required(&self, argument: Argument) -> Result<&[u8], UsageError> {
        self.bytes(argument)
            .ok_or_else(|| UsageError::new(Some(argument), UsageReason::Missing))
    }

    /// The JSON bytes of the options of `argument`, when given.
    pub(crate) fn options(&self, argument: Argument) -> Option<&[u8]> {
        match self.members.get(&argument) {
            Some(MemberValue::Options(bytes)) => Some(bytes),
            _ => None,
        }
    }
}

/// The tool call of the tool named `name` with `arguments`; absent arguments are an empty
/// object.
///
/// # Errors
///
/// The usage error of an unknown tool, arguments that are not an object, an unknown member, a
/// document or name that is not a string, or a missing required member.
pub(crate) fn parse_call(name: &str, arguments: Option<&Value>) -> Result<ToolCall, UsageError> {
    let command = CommandName::from_name(name)?;
    let members = match arguments {
        None => serde_json::Map::new(),
        Some(Value::Object(members)) => members.clone(),
        Some(_) => return Err(UsageError::new(None, UsageReason::ArgumentsNotAnObject)),
    };
    let mut values = BTreeMap::new();
    for (member, value) in &members {
        let argument = member_argument(command, member)?;
        values.insert(argument, member_value(argument, value)?);
    }
    let present: Vec<Argument> = values.keys().copied().collect();
    command.check_required(&present)?;
    Ok(ToolCall {
        command,
        members: values,
    })
}

/// The argument of the member named `member`, when the tool takes it.
fn member_argument(command: CommandName, member: &str) -> Result<Argument, UsageError> {
    command
        .argument(member)
        .ok()
        .filter(|argument| has_member(command, *argument))
        .ok_or_else(|| UsageError::new(None, UsageReason::UnknownArgument(member.to_owned())))
}

/// The value of the member of `argument`. An options value of any type is kept as JSON, so
/// `core` refuses one that is not an object with the CLI's reason; a document or name must be
/// a string.
fn member_value(argument: Argument, value: &Value) -> Result<MemberValue, UsageError> {
    match (member_kind(argument), value) {
        (MemberKind::Options, value) => Ok(MemberValue::Options(value.to_string().into_bytes())),
        (MemberKind::Document | MemberKind::Name, Value::String(text)) => {
            Ok(MemberValue::Text(text.clone()))
        }
        (MemberKind::Document | MemberKind::Name, _) => {
            Err(UsageError::new(Some(argument), UsageReason::NotAString))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn reason(name: &str, arguments: &Value) -> UsageReason {
        parse_call(name, Some(arguments)).unwrap_err().reason
    }

    #[test]
    fn arc_6_7_an_unknown_tool_is_a_usage_error() {
        assert_eq!(
            reason("run", &json!({})),
            UsageReason::UnknownCommand("run".to_owned())
        );
    }

    #[test]
    fn arc_6_7_compile_has_no_audit_record_member() {
        let arguments = json!({"model": "", "catalog_schema": "", "query": "",
            "security_context": "", "audit_record": "a.json"});
        assert_eq!(
            reason("compile", &arguments),
            UsageReason::UnknownArgument("audit_record".to_owned())
        );
    }

    #[test]
    fn arc_6_7_a_document_of_the_wrong_type_or_null_is_a_usage_error() {
        for value in [json!(null), json!(1), json!({})] {
            let error = parse_call(
                "validate",
                Some(&json!({"model": value, "catalog_schema": ""})),
            )
            .unwrap_err();
            assert_eq!(
                error,
                UsageError::new(Some(Argument::Model), UsageReason::NotAString)
            );
        }
    }

    #[test]
    fn arc_6_7_options_of_any_type_go_to_core() {
        let call = parse_call(
            "validate",
            Some(&json!({"model": "m", "catalog_schema": "c", "load_options": null})),
        )
        .unwrap();
        assert_eq!(call.options(Argument::LoadOptions), Some(&b"null"[..]));
    }

    #[test]
    fn arc_6_7_absent_arguments_miss_the_first_required_member() {
        assert_eq!(
            parse_call("migrate", None).unwrap_err(),
            UsageError::new(Some(Argument::Model), UsageReason::Missing)
        );
        assert_eq!(
            reason("migrate", &json!([])),
            UsageReason::ArgumentsNotAnObject
        );
    }
}
