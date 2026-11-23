//! ARC-6.9: the crate documentation lists the tool names, input schemas and descriptions
//! exactly as `tools/list` returns them.

use crate::tools::tool_list;

/// The JSON block of the README that follows the "Tools" heading.
fn documented_tools() -> serde_json::Value {
    let readme = include_str!("../README.md");
    let block = readme
        .split("```json\n")
        .nth(1)
        .and_then(|rest| rest.split("```").next())
        .expect("the README lists the tools in a JSON block");
    serde_json::from_str(block).expect("the README's JSON block is JSON")
}

#[test]
fn arc_6_9_the_documentation_lists_every_tool_with_its_schema_and_description() {
    assert_eq!(
        documented_tools(),
        tool_list(),
        "README.md must hold:\n{}",
        serde_json::to_string_pretty(&tool_list()).unwrap()
    );
}
