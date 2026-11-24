//! Contract versions in golden JSON files (CS-2.10b): a golden file writes `@schema` where the
//! produced value holds the version of a contract the compiler produces, and the runner puts
//! the version of that contract's schema in its place before comparing (CS-2.10).

use std::convert::Infallible;

use menura_contracts::ContractKind;

use crate::canonical_json::JsonValue;

/// The string a golden file writes in place of a produced contract version (CS-2.10b).
pub(crate) const SCHEMA_PLACEHOLDER: &str = "@schema";

/// The member of a document that holds its contract version.
const CONTRACT_VERSION: &str = "contract_version";

/// The member that names a contract, in a document and in a capability entry (DSC-4).
const CONTRACT: &str = "contract";

/// The member of a model description that holds the capability profile (DSC-4).
const CAPABILITIES: &str = "capabilities";

/// The member of the capability profile that lists the contracts (DSC-4).
const CONTRACTS: &str = "contracts";

/// The member of a capability entry that holds the contract version (DSC-4).
const VERSION: &str = "version";

/// The golden value `golden` with every `@schema` replaced by the version of the schema of the
/// contract it belongs to (CS-2.10b): the document's `contract_version`, named by its
/// `contract`, and the `version` of each entry of `capabilities.contracts[]`, named by the
/// entry's `contract`. It fails, naming `file_name`, on an `@schema` anywhere else or one whose
/// contract is unknown.
pub(crate) fn with_schema_versions(
    golden: &JsonValue,
    file_name: &str,
) -> Result<JsonValue, String> {
    let resolved = map_version_places(golden, &|entry, member| -> Result<JsonValue, String> {
        if member != &JsonValue::String(SCHEMA_PLACEHOLDER.to_owned()) {
            return Ok(member.clone());
        }
        let contract = contract_named_by(entry).ok_or_else(|| {
            format!(
                "the golden file `{file_name}` writes `{SCHEMA_PLACEHOLDER}` for a contract \
                 that is not known (CS-2.10b)"
            )
        })?;
        Ok(JsonValue::String(contract.latest_version().to_string()))
    })?;
    if holds_placeholder(&resolved) {
        return Err(format!(
            "the golden file `{file_name}` writes `{SCHEMA_PLACEHOLDER}` where no produced \
             contract version is (CS-2.10b)"
        ));
    }
    Ok(resolved)
}

/// The produced value `produced` as a golden file stores it (CS-2.10b): the document's
/// `contract_version` and the `version` of each entry of `capabilities.contracts[]` are
/// written `@schema`.
pub(crate) fn with_schema_placeholders(produced: &JsonValue) -> JsonValue {
    let stored = map_version_places(produced, &|_, _| {
        Ok::<_, Infallible>(JsonValue::String(SCHEMA_PLACEHOLDER.to_owned()))
    });
    match stored {
        Ok(stored) => stored,
        Err(never) => match never {},
    }
}

/// The value with `replace(entry, member)` in place of each member that holds a produced
/// contract version: the top-level `contract_version`, and the `version` of each entry of the
/// top-level `capabilities.contracts[]`.
fn map_version_places<E>(
    document: &JsonValue,
    replace: &impl Fn(&JsonValue, &JsonValue) -> Result<JsonValue, E>,
) -> Result<JsonValue, E> {
    let JsonValue::Object(members) = document else {
        return Ok(document.clone());
    };
    let mut mapped = Vec::with_capacity(members.len());
    for (name, member) in members {
        let value = match name.as_str() {
            CONTRACT_VERSION => replace(document, member)?,
            CAPABILITIES => map_capabilities(member, replace)?,
            _ => member.clone(),
        };
        mapped.push((name.clone(), value));
    }
    Ok(JsonValue::Object(mapped))
}

/// The capability profile with `replace(entry, version)` in place of the `version` of each
/// entry of `contracts[]` (DSC-4).
fn map_capabilities<E>(
    capabilities: &JsonValue,
    replace: &impl Fn(&JsonValue, &JsonValue) -> Result<JsonValue, E>,
) -> Result<JsonValue, E> {
    let JsonValue::Object(members) = capabilities else {
        return Ok(capabilities.clone());
    };
    let mut mapped = Vec::with_capacity(members.len());
    for (name, member) in members {
        let value = match (name.as_str(), member) {
            (CONTRACTS, JsonValue::Array(entries)) => JsonValue::Array(
                entries
                    .iter()
                    .map(|entry| map_entry_version(entry, replace))
                    .collect::<Result<_, E>>()?,
            ),
            _ => member.clone(),
        };
        mapped.push((name.clone(), value));
    }
    Ok(JsonValue::Object(mapped))
}

/// The capability entry with `replace(entry, version)` in place of its `version`.
fn map_entry_version<E>(
    entry: &JsonValue,
    replace: &impl Fn(&JsonValue, &JsonValue) -> Result<JsonValue, E>,
) -> Result<JsonValue, E> {
    let JsonValue::Object(members) = entry else {
        return Ok(entry.clone());
    };
    let mut mapped = Vec::with_capacity(members.len());
    for (name, member) in members {
        let value = if name == VERSION {
            replace(entry, member)?
        } else {
            member.clone()
        };
        mapped.push((name.clone(), value));
    }
    Ok(JsonValue::Object(mapped))
}

/// The contract whose schema an object's `contract` names, as the capability profile maps
/// them (DSC-4): a contract by its name or its former name (CON-0.5), `menura.explanation` to
/// the compiled plan and `menura.validation_report` to the compile error.
fn contract_named_by(object: &JsonValue) -> Option<ContractKind> {
    let JsonValue::String(name) = object.member(CONTRACT)? else {
        return None;
    };
    match name.as_str() {
        "menura.explanation" => Some(ContractKind::CompiledPlan),
        "menura.validation_report" => Some(ContractKind::CompileError),
        _ => ContractKind::ALL
            .into_iter()
            .find(|kind| name == kind.contract_name() || name == kind.former_contract_name()),
    }
}

/// Whether an `@schema` string is somewhere in `value`.
fn holds_placeholder(value: &JsonValue) -> bool {
    match value {
        JsonValue::String(text) => text == SCHEMA_PLACEHOLDER,
        JsonValue::Array(elements) => elements.iter().any(holds_placeholder),
        JsonValue::Object(members) => members.iter().any(|(_, member)| holds_placeholder(member)),
        JsonValue::Null | JsonValue::Bool(_) | JsonValue::Number(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn json(text: &str) -> JsonValue {
        JsonValue::parse(text).unwrap()
    }

    fn version(kind: ContractKind) -> String {
        kind.latest_version().to_string()
    }

    #[test]
    fn cs_2_10b_the_document_version_is_the_schema_version() {
        let golden = json(r#"{"contract": "menura.audit_record", "contract_version": "@schema"}"#);
        let resolved = with_schema_versions(&golden, "g.json").unwrap();
        assert_eq!(
            resolved.member(CONTRACT_VERSION),
            Some(&JsonValue::String(version(ContractKind::AuditRecord)))
        );
    }

    #[test]
    fn cs_2_10b_capability_versions_are_the_schema_versions() {
        let golden = json(
            r#"{"capabilities": {"contracts": [
                {"contract": "menura.explanation", "version": "@schema"},
                {"contract": "menura.query", "version": "@schema"},
                {"contract": "menura.validation_report", "version": "@schema"}
            ]}}"#,
        );
        let expected = json(&format!(
            r#"{{"capabilities": {{"contracts": [
                {{"contract": "menura.explanation", "version": "{}"}},
                {{"contract": "menura.query", "version": "{}"}},
                {{"contract": "menura.validation_report", "version": "{}"}}
            ]}}}}"#,
            version(ContractKind::CompiledPlan),
            version(ContractKind::Query),
            version(ContractKind::CompileError)
        ));
        assert_eq!(with_schema_versions(&golden, "g.json").unwrap(), expected);
    }

    #[test]
    fn cs_2_10b_a_former_contract_name_maps_to_its_contract() {
        let golden = json(
            r#"{"contract": "semantic_compiler.compile_error", "contract_version": "@schema"}"#,
        );
        let resolved = with_schema_versions(&golden, "g.json").unwrap();
        assert_eq!(
            resolved.member(CONTRACT_VERSION),
            Some(&JsonValue::String(version(ContractKind::CompileError)))
        );
    }

    #[test]
    fn cs_2_10b_a_literal_version_is_kept() {
        let golden = json(r#"{"contract": "menura.audit_record", "contract_version": "1.0.0"}"#);
        assert_eq!(with_schema_versions(&golden, "g.json").unwrap(), golden);
    }

    #[test]
    fn cs_2_10b_a_placeholder_elsewhere_fails() {
        let golden = json(
            r#"{"contract": "menura.audit_record", "contract_version": "@schema",
                "contract_versions": {"query": "@schema"}}"#,
        );
        let reason = with_schema_versions(&golden, "g.json").unwrap_err();
        assert!(reason.contains("g.json"), "{reason}");
        assert!(reason.contains("CS-2.10b"), "{reason}");
    }

    #[test]
    fn cs_2_10b_a_nested_document_version_placeholder_fails() {
        let golden =
            json(r#"{"inner": {"contract": "menura.query", "contract_version": "@schema"}}"#);
        assert!(with_schema_versions(&golden, "g.json").is_err());
    }

    #[test]
    fn cs_2_10b_a_placeholder_for_an_unknown_contract_fails() {
        for golden in [
            r#"{"contract": "menura.unknown", "contract_version": "@schema"}"#,
            r#"{"contract_version": "@schema"}"#,
            r#"{"capabilities": {"contracts": [{"version": "@schema"}]}}"#,
        ] {
            let reason = with_schema_versions(&json(golden), "g.json").unwrap_err();
            assert!(reason.contains("not known"), "{reason}");
        }
    }

    #[test]
    fn cs_2_10b_produced_versions_are_stored_as_placeholders() {
        let produced = json(
            r#"{"capabilities": {"contracts": [{"contract": "menura.query", "version": "1.1.0"}]},
                "contract": "menura.model_description", "contract_version": "1.2.0",
                "other": "1.2.0"}"#,
        );
        let expected = json(
            r#"{"capabilities": {"contracts": [{"contract": "menura.query", "version": "@schema"}]},
                "contract": "menura.model_description", "contract_version": "@schema",
                "other": "1.2.0"}"#,
        );
        assert_eq!(with_schema_placeholders(&produced), expected);
    }
}
