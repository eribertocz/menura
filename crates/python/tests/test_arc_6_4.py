"""ARC-6.4: documents as `bytes` or `str`, options as `dict` read as the CLI reads its files."""

import unittest

import menura
from support import QUERY, analyst, slice_file, slice_handle

LOAD_OPTIONS = "`load_options`: "
COMPILE_OPTIONS = "`compile_options`: "


class Arc64(unittest.TestCase):
    def test_arc_6_4_a_str_document_equals_its_utf8_bytes(self):
        model, schema = slice_file("model.yaml"), slice_file("catalog-schema.json")
        as_bytes = menura.validate(model, schema)
        as_text = menura.validate(model.decode("utf-8"), schema.decode("utf-8"))
        self.assertEqual(as_bytes.document, as_text.document)

    def test_arc_6_4_content_problems_are_documents_not_exceptions(self):
        result = menura.validate(b"\xff not yaml", b"{}")
        self.assertTrue(result.is_error)
        self.assertIn("menura.validation_report", result.document)

    def test_arc_6_4_a_document_of_another_type_is_a_type_error(self):
        with self.assertRaises(TypeError):
            menura.validate(1, b"{}")

    def test_arc_6_4_a_bool_is_only_a_boolean(self):
        handle = slice_handle()
        with self.assertRaisesRegex(ValueError, "`timeout_ms` must be a positive integer"):
            handle.compile(QUERY, analyst(), {"timeout_ms": True})
        with self.assertRaisesRegex(ValueError, "`audit` must be a boolean"):
            handle.compile(QUERY, analyst(), {"audit": 1})
        audited = handle.compile(QUERY, analyst(), {"audit": True})
        self.assertIsNotNone(audited.audit_record)

    def test_arc_6_4_an_int_is_an_integer_and_a_float_is_not(self):
        handle = slice_handle()
        plan = handle.compile(QUERY, analyst(), {"timeout_ms": 5000, "max_rows": 10})
        self.assertFalse(plan.is_error)
        self.assertIn('"timeout_ms": 5000', plan.document)
        with self.assertRaisesRegex(ValueError, "`timeout_ms` must be a positive integer"):
            handle.compile(QUERY, analyst(), {"timeout_ms": 5000.0})

    def test_arc_6_4_an_unknown_option_raises_value_error(self):
        with self.assertRaises(ValueError) as raised:
            menura.validate(b"", b"{}", {"colour": "red"})
        self.assertTrue(str(raised.exception).startswith(LOAD_OPTIONS))
        self.assertIn("`colour`", str(raised.exception))

    def test_arc_6_4_a_none_value_raises_value_error(self):
        with self.assertRaises(ValueError) as raised:
            menura.validate(b"", b"{}", {"namespace": None})
        self.assertTrue(str(raised.exception).startswith(LOAD_OPTIONS))

    def test_arc_6_4_a_key_that_is_not_a_str_raises_value_error(self):
        with self.assertRaises(ValueError) as raised:
            slice_handle().compile(QUERY, analyst(), {1: "duckdb"})
        self.assertEqual(
            str(raised.exception), COMPILE_OPTIONS + "the compile options are not a JSON object"
        )

    def test_arc_6_4_options_that_are_not_a_dict_raise_value_error(self):
        for options in ([], "{}", 3, ("dialect", "duckdb")):
            with self.subTest(options=options), self.assertRaises(ValueError) as raised:
                menura.migrate(b"", options)
            self.assertEqual(
                str(raised.exception), LOAD_OPTIONS + "the load options are not a JSON object"
            )

    def test_arc_6_4_an_optional_argument_passed_as_none_is_absent(self):
        model, schema = slice_file("model.yaml"), slice_file("catalog-schema.json")
        self.assertEqual(
            menura.validate(model, schema, None).document,
            menura.validate(model, schema).document,
        )
        handle = menura.Handle.load(model, schema, None, None)
        self.assertEqual(
            handle.compile(QUERY, analyst(), None).document,
            handle.compile(QUERY, analyst()).document,
        )
        self.assertEqual(
            handle.integrity_checks(None).document, handle.integrity_checks().document
        )


if __name__ == "__main__":
    unittest.main()
