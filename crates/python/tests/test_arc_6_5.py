"""ARC-6.5: every call returns a `Result`; `Handle.load` raises `CompileError`."""

import json
import unittest

import menura
from support import QUERY, analyst, slice_file, slice_handle


class Arc65(unittest.TestCase):
    def test_arc_6_5_validate_returns_the_report_and_its_error_flag(self):
        valid = menura.validate(slice_file("model.yaml"), slice_file("catalog-schema.json"))
        self.assertIsInstance(valid, menura.Result)
        self.assertFalse(valid.is_error)
        self.assertIsNone(valid.audit_record)
        self.assertEqual(json.loads(valid.document)["contract"], "menura.validation_report")
        self.assertFalse(valid.document.endswith("\n"))
        invalid = menura.validate(b"", b"{}")
        self.assertTrue(invalid.is_error)

    def test_arc_6_5_migrate_returns_the_migration_report(self):
        result = menura.migrate(slice_file("model.yaml"))
        self.assertFalse(result.is_error)
        self.assertEqual(json.loads(result.document)["contract"], "menura.migration_report")
        self.assertTrue(menura.migrate(b"").is_error)

    def test_arc_6_5_diff_compares_two_models(self):
        model, schema = slice_file("model.yaml"), slice_file("catalog-schema.json")
        result = menura.diff(model, schema, model, schema)
        self.assertFalse(result.is_error)
        self.assertEqual(json.loads(result.document)["contract"], "menura.model_diff")
        self.assertTrue(menura.diff(b"", b"{}", model, schema).is_error)

    def test_arc_6_5_load_raises_compile_error_with_the_document(self):
        with self.assertRaises(menura.CompileError) as raised:
            menura.Handle.load(b"", b"{}")
        document = json.loads(raised.exception.document)
        self.assertEqual(document["contract"], "menura.compile_error")
        self.assertTrue(document["errors"])

    def test_arc_6_5_the_handle_warnings_are_a_json_array(self):
        handle = slice_handle()
        self.assertIsInstance(json.loads(handle.warnings), list)
        plan = json.loads(handle.compile(QUERY, analyst()).document)
        self.assertEqual(json.loads(handle.warnings), plan["warnings"])

    def test_arc_6_5_with_state_appends_the_state_warning(self):
        handle = slice_handle()
        stated = handle.with_state(b"not json")
        self.assertIsInstance(stated, menura.Handle)
        load_warnings = json.loads(handle.warnings)
        warnings = json.loads(stated.warnings)
        self.assertEqual(warnings[: len(load_warnings)], load_warnings)
        self.assertTrue(warnings[len(load_warnings)]["code"].startswith("WCAT-"))
        self.assertEqual(json.loads(handle.warnings), load_warnings)

    def test_arc_6_5_compile_returns_the_plan_and_the_audit_record_when_asked(self):
        handle = slice_handle()
        plan = handle.compile(QUERY, analyst())
        self.assertFalse(plan.is_error)
        self.assertEqual(json.loads(plan.document)["contract"], "menura.compiled_plan")
        self.assertIsNone(plan.audit_record)
        audited = handle.compile(QUERY, analyst(), {"audit": True, "actor": "agent-7"})
        self.assertEqual(json.loads(audited.audit_record)["contract"], "menura.audit_record")
        refused = handle.compile(b"{}", analyst(), {"audit": True})
        self.assertTrue(refused.is_error)
        self.assertIsNotNone(refused.audit_record)

    def test_arc_6_5_explain_returns_the_explanation(self):
        result = slice_handle().explain(QUERY, analyst())
        self.assertFalse(result.is_error)
        document = json.loads(result.document)
        self.assertEqual(document["contract"], "menura.explanation")
        self.assertEqual(document["sql"], "")
        self.assertIsNone(result.audit_record)

    def test_arc_6_5_describe_integrity_checks_and_dependencies(self):
        handle = slice_handle()
        description = handle.describe(analyst())
        self.assertFalse(description.is_error)
        self.assertEqual(
            json.loads(description.document)["contract"], "menura.model_description"
        )
        checks = handle.integrity_checks()
        self.assertEqual(json.loads(checks.document)["contract"], "menura.integrity_checks")
        self.assertEqual(handle.integrity_checks("duckdb").document, checks.document)
        with self.assertRaisesRegex(ValueError, "^unknown dialect `postgres`; the dialect is `duckdb`$"):
            handle.integrity_checks("postgres")
        dependencies = handle.dependencies()
        self.assertEqual(
            json.loads(dependencies.document)["contract"], "menura.model_dependencies"
        )

    def test_arc_6_5_the_handle_is_immutable(self):
        handle = slice_handle()
        with self.assertRaises(AttributeError):
            handle.warnings = "[]"


if __name__ == "__main__":
    unittest.main()
