"""Unit tests of the contract change check. Run: python3 -m unittest discover -s scripts/ci -p 'test_contract_changes.py'"""
import copy
import json
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import check_contract_changes  # noqa: E402

SCHEMA_FILE = "query.schema.json"
EXAMPLE_FILE = "query.json-form.example.json"


def schema(version, properties=None, required=None):
    """Build a synthetic closed schema of the query contract at the given version."""
    return {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$id": f"https://menura.invalid/spec/contracts/query/{version}",
        "type": "object",
        "additionalProperties": False,
        "required": required if required is not None else ["contract_version"],
        "properties": properties
        if properties is not None
        else {"contract_version": {"type": "string"}, "limit": {"type": ["integer", "null"]}},
    }


def problems(before, after, example_changed=True):
    """Run the check on one schema, with its example changed or not."""
    head_example = '{"changed": true}' if example_changed else "{}"
    return check_contract_changes.find_problems(
        {SCHEMA_FILE: json.dumps(before)},
        {SCHEMA_FILE: json.dumps(after)},
        {EXAMPLE_FILE: "{}"},
        {EXAMPLE_FILE: head_example},
    )


class ContractChangesTest(unittest.TestCase):
    """A3, A5, CON-3."""

    def test_a5_unchanged_schema_passes_without_bump_or_example(self):
        self.assertEqual(problems(schema("1.1.0"), schema("1.1.0"), example_changed=False), [])

    def test_a5_missing_bump_fails(self):
        after = schema("1.1.0")
        after["properties"]["offset"] = {"type": "integer"}
        self.assertEqual(len(problems(schema("1.1.0"), after)), 1)
        self.assertIn("at least a minor bump", problems(schema("1.1.0"), after)[0])

    def test_a5_patch_bump_on_additive_change_fails(self):
        after = schema("1.1.1")
        after["properties"]["offset"] = {"type": "integer"}
        self.assertIn("at least a minor bump", problems(schema("1.1.0"), after)[0])

    def test_con3_minor_bump_on_additive_optional_change_passes(self):
        after = schema("1.2.0")
        after["properties"]["offset"] = {"type": "integer"}
        self.assertEqual(problems(schema("1.1.0"), after), [])

    def test_con3_minor_bump_on_removed_member_fails(self):
        after = schema("1.2.0", properties={"contract_version": {"type": "string"}})
        self.assertIn("major bump", problems(schema("1.1.0"), after)[0])
        self.assertIn("member limit removed", problems(schema("1.1.0"), after)[0])

    def test_con3_minor_bump_on_narrowed_type_fails(self):
        after = schema("1.2.0")
        after["properties"]["limit"] = {"type": "integer"}
        self.assertIn("type narrowed", problems(schema("1.1.0"), after)[0])

    def test_con3_minor_bump_on_new_required_member_fails(self):
        after = schema("1.2.0", required=["contract_version", "limit"])
        self.assertIn("new required member limit", problems(schema("1.1.0"), after)[0])

    def test_con3_major_bump_on_breaking_change_passes(self):
        after = schema("2.0.0", required=["contract_version", "limit"])
        self.assertEqual(problems(schema("1.1.0"), after), [])

    def test_con3_breaking_change_in_nested_definition_is_found(self):
        before = schema("1.1.0")
        before["$defs"] = {"item": {"type": "object", "properties": {"name": {"type": "string"}}}}
        after = copy.deepcopy(before)
        after["$id"] = "https://menura.invalid/spec/contracts/query/1.2.0"
        after["$defs"]["item"]["properties"] = {}
        self.assertIn("#/$defs/item: member name removed", problems(before, after)[0])

    def test_con3_widened_type_is_not_breaking(self):
        before = schema("1.1.0")
        before["properties"]["limit"] = {"type": "integer"}
        self.assertEqual(problems(before, schema("1.2.0")), [])

    def test_a5_changed_schema_with_unchanged_example_fails(self):
        after = schema("1.2.0")
        after["properties"]["offset"] = {"type": "integer"}
        self.assertEqual(
            problems(schema("1.1.0"), after, example_changed=False),
            ["query: schema changed but no example of this contract changed"],
        )

    def test_a5_new_schema_needs_an_example(self):
        result = check_contract_changes.find_problems({}, {SCHEMA_FILE: json.dumps(schema("1.0.0"))}, {}, {})
        self.assertEqual(result, ["query: schema changed but no example of this contract changed"])

    def test_a3_invalid_schema_fails(self):
        after = schema("1.1.0")
        after["type"] = "no-such-type"
        self.assertIn("not a valid JSON Schema", problems(schema("1.1.0"), after)[0])

    def test_a3_unparsable_schema_fails(self):
        result = check_contract_changes.find_problems({}, {SCHEMA_FILE: "{"}, {}, {})
        self.assertIn("not a valid JSON Schema", result[0])

    def test_a5_missing_version_in_id_fails(self):
        after = schema("1.2.0")
        after["$id"] = "https://menura.invalid/spec/contracts/query"
        after["properties"]["offset"] = {"type": "integer"}
        self.assertIn("does not end with", problems(schema("1.1.0"), after)[0])


if __name__ == "__main__":
    unittest.main()
