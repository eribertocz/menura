"""Tests of check_wrapper_parity.py (ARC-6.9): an injected difference is reported."""

import pathlib
import sys
import types
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import check_wrapper_parity  # noqa: E402

DOCUMENT = '{\n  "sql": "SELECT 1"\n}'


def fake_package(document):
    """A stand-in for `menura` whose `validate` returns `document`."""

    class CompileError(Exception):
        document = ""

    def validate(model, catalog_schema, load_options):
        if load_options == {"colour": 1}:
            raise ValueError("unknown load option `colour`")
        return types.SimpleNamespace(document=document, is_error=False, audit_record=None)

    return types.SimpleNamespace(CompileError=CompileError, validate=validate)


def manifest(expected, options=None):
    arguments = {"model": {"text": "m"}, "catalog_schema": {"text": "{}"}}
    if options is not None:
        arguments["load_options"] = {"options": options}
    call = {"case": "c-001", "command": "validate", "arguments": arguments, "expected": expected}
    return {"calls": [call]}


OUTPUT = {"document": DOCUMENT, "is_error": False, "audit_record": None}


class WrapperParity(unittest.TestCase):
    def test_arc_6_9_equal_documents_have_no_difference(self):
        self.assertEqual(check_wrapper_parity.check(manifest(OUTPUT), fake_package(DOCUMENT)), [])

    def test_arc_6_9_an_injected_difference_reports_case_wrapper_and_first_line(self):
        changed = DOCUMENT.replace("SELECT 1", "SELECT 2")
        found = check_wrapper_parity.check(manifest(OUTPUT), fake_package(changed))
        self.assertEqual(
            found,
            [
                "case c-001: python `validate`: document: line 2, column 18: "
                'expected `  "sql": "SELECT 1"`, found `  "sql": "SELECT 2"`'
            ],
        )

    def test_arc_6_9_a_usage_error_message_is_compared(self):
        expected = {"usage_error": "unknown load option `color`"}
        found = check_wrapper_parity.check(
            manifest(expected, {"colour": 1}), fake_package(DOCUMENT)
        )
        self.assertEqual(len(found), 1)
        self.assertIn("ValueError: line 1, column 26", found[0])

    def test_arc_6_9_a_missing_final_line_is_a_difference(self):
        self.assertEqual(
            check_wrapper_parity.first_difference("a\n", "a"),
            "line 2, column 1: expected ``, found no line",
        )


if __name__ == "__main__":
    unittest.main()
