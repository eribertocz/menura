"""A4 and Q1 for the crate `python`: it depends only on `core` and forbids unsafe code."""

import re
import unittest

from support import ROOT

CRATE = ROOT / "crates" / "python"


class A4Q1(unittest.TestCase):
    def test_a4_the_only_workspace_dependency_is_core(self):
        manifest = (CRATE / "Cargo.toml").read_text(encoding="utf-8")
        dependencies = manifest.split("[dependencies]", 1)[1].split("\n[", 1)[0]
        workspace = re.findall(r"^(menura-[a-z0-9-]+)", dependencies, re.MULTILINE)
        self.assertEqual(workspace, ["menura-core"])

    def test_q1_the_crate_root_forbids_unsafe_code(self):
        root = (CRATE / "src" / "lib.rs").read_text(encoding="utf-8")
        self.assertIn("#![forbid(unsafe_code)]", root)


if __name__ == "__main__":
    unittest.main()
