"""Unit tests of the unsafe check. Run: python3 -m unittest discover -s scripts/ci -p 'test_unsafe.py'"""
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import check_unsafe  # noqa: E402


def metadata(targets):
    """Build a minimal `cargo metadata --no-deps` document with one package and the given targets."""
    return {
        "packages": [
            {
                "id": "id-a",
                "name": "a",
                "targets": [{"kind": kinds, "src_path": path} for kinds, path in targets],
            }
        ],
        "workspace_members": ["id-a"],
    }


class UnsafeTest(unittest.TestCase):
    """Q1."""

    def test_q1_crate_roots_are_library_binary_and_procedural_macro_targets(self):
        document = metadata(
            [(["lib"], "/a/lib.rs"), (["bin"], "/a/main.rs"), (["proc-macro"], "/a/macro.rs"), (["test"], "/a/t.rs")]
        )
        self.assertEqual(check_unsafe.crate_roots(document), ["/a/lib.rs", "/a/main.rs", "/a/macro.rs"])

    def test_q1_root_with_attribute_passes(self):
        self.assertTrue(check_unsafe.forbids_unsafe("//! Docs.\n\n#![forbid(unsafe_code)]\n"))

    def test_q1_root_with_spaced_attribute_passes(self):
        self.assertTrue(check_unsafe.forbids_unsafe("#![ forbid( unsafe_code ) ]\n"))

    def test_q1_root_without_attribute_fails(self):
        self.assertFalse(check_unsafe.forbids_unsafe("//! Docs.\n"))

    def test_q1_weaker_attribute_fails(self):
        self.assertFalse(check_unsafe.forbids_unsafe("#![deny(unsafe_code)]\n"))

    def test_q1_commented_attribute_fails(self):
        self.assertFalse(check_unsafe.forbids_unsafe("// #![forbid(unsafe_code)]\n"))

    def test_q1_attribute_in_a_string_fails(self):
        self.assertFalse(check_unsafe.forbids_unsafe('const A: &str = "#![forbid(unsafe_code)]";\n'))


if __name__ == "__main__":
    unittest.main()
