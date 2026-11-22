//! The rewrite `MIG-3.2a`: an extension entry under the former namespace `semantic_compiler`
//! gets the configured namespace (CON-0.5).

use menura_contracts::FORMER_NAMESPACE;
use menura_contracts::types::audit_record::OssieFormat;
use menura_contracts::types::migration_report::{MigrationChange, MigrationRule};
use menura_contracts::types::ordered_json::{OrderedJson, OrderedObject};
use serde_json::Value;

use super::data_text::top_level_members;
use super::location::Location;

/// The `vendor_name` of the entries the compiler reads (E0C-1, condition 1).
const COMMON: &str = "COMMON";

/// The member of `data` that names the namespace (MDL-2.2).
const NAMESPACE: &str = "namespace";

/// Rewrites `entry`, the extension entry at `location` in `format`, when its `data` has exactly
/// one `namespace` member, the string `semantic_compiler`; returns the position of the
/// rewritten member and its change. Every other entry is left unchanged (MIG-3.2a).
pub(super) fn rewrite_former_namespace(
    entry: &mut OrderedJson,
    location: &Location,
    format: OssieFormat,
    namespace: &str,
) -> Option<(Vec<usize>, MigrationChange)> {
    let OrderedJson::Object(entry) = entry else {
        return None;
    };
    if entry.get("vendor_name") != Some(&OrderedJson::String(COMMON.to_owned())) {
        return None;
    }
    let (index, data) = entry
        .members_mut()
        .iter_mut()
        .enumerate()
        .find(|(_, (name, _))| name == "data")
        .map(|(index, (_, data))| (index, data))?;
    let data_location = location.member("data", index);
    let rewritten = match (format, data) {
        (OssieFormat::Release, OrderedJson::String(text)) => {
            rewrite_text(text, namespace).map(|()| data_location)
        }
        (OssieFormat::Development, OrderedJson::Object(object)) => {
            rewrite_object(object, namespace).map(|index| data_location.member(NAMESPACE, index))
        }
        _ => None,
    }?;
    let change = MigrationChange {
        pointer: rewritten.pointer,
        rule: MigrationRule::FormerNamespace,
        from: FORMER_NAMESPACE.to_owned(),
        to: namespace.to_owned(),
    };
    Some((rewritten.position, change))
}

/// Replaces, in the JSON text `text` of a release-format `data`, the string token of its one
/// `namespace` value `semantic_compiler` by the JSON string of `namespace`, keeping every other
/// character (MIG-3.2a). `None`, and `text` unchanged, when the entry is not rewritten.
fn rewrite_text(text: &mut String, namespace: &str) -> Option<()> {
    if !matches!(serde_json::from_str::<Value>(text), Ok(Value::Object(_))) {
        return None;
    }
    let members = top_level_members(text)?;
    let mut namespaces = members.iter().filter(|member| member.name == NAMESPACE);
    let (Some(member), None) = (namespaces.next(), namespaces.next()) else {
        return None;
    };
    let token = text.get(member.value.clone())?;
    if serde_json::from_str::<Value>(token).ok()? != Value::String(FORMER_NAMESPACE.to_owned()) {
        return None;
    }
    let replacement = serde_json::to_string(namespace).ok()?;
    text.replace_range(member.value.clone(), &replacement);
    Some(())
}

/// Sets the one `namespace` member of a development-format `data` object to `namespace` when
/// it is the string `semantic_compiler`, and returns its index (MIG-3.2a).
fn rewrite_object(object: &mut OrderedObject, namespace: &str) -> Option<usize> {
    let mut namespaces = object
        .members_mut()
        .iter_mut()
        .enumerate()
        .filter(|(_, (name, _))| name == NAMESPACE);
    let (Some((index, (_, value))), None) = (namespaces.next(), namespaces.next()) else {
        return None;
    };
    if *value != OrderedJson::String(FORMER_NAMESPACE.to_owned()) {
        return None;
    }
    *value = OrderedJson::String(namespace.to_owned());
    Some(index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rewritten(text: &str) -> Option<String> {
        let mut text = text.to_owned();
        rewrite_text(&mut text, "menura").map(|()| text)
    }

    #[test]
    fn mig_3_2a_only_the_namespace_token_changes() {
        assert_eq!(
            rewritten(
                r#" { "version":1 ,"namespace" :  "semantic_compiler", "x": "semantic_compiler"}"#
            ),
            Some(
                r#" { "version":1 ,"namespace" :  "menura", "x": "semantic_compiler"}"#.to_owned()
            )
        );
    }

    #[test]
    fn mig_3_2a_an_escaped_former_namespace_is_rewritten() {
        assert_eq!(
            rewritten(r#"{"namespace": "semantic_compiler"}"#),
            Some(r#"{"namespace": "menura"}"#.to_owned())
        );
    }

    #[test]
    fn mig_3_2a_other_entries_are_left_unchanged() {
        for text in [
            r#"{"namespace": "menura"}"#,
            r#"{"namespace": "semantic_compiler", "namespace": "semantic_compiler"}"#,
            r#"{"inner": {"namespace": "semantic_compiler"}}"#,
            r#"["semantic_compiler"]"#,
            r#"{"namespace": "semantic_compiler""#,
            r#"{"namespace": ["semantic_compiler"]}"#,
        ] {
            assert_eq!(rewritten(text), None, "{text}");
        }
    }
}
