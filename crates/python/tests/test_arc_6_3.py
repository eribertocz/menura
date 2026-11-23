"""ARC-6.3: the package `menura`, one `abi3` wheel with type stubs and a `py.typed` marker."""

import importlib.resources
import pathlib
import unittest

import menura


class Arc63(unittest.TestCase):
    def test_arc_6_3_the_import_name_is_menura(self):
        self.assertEqual(menura.__name__, "menura")
        for name in ("validate", "migrate", "diff", "Handle", "Result", "CompileError"):
            self.assertTrue(hasattr(menura, name), name)

    def test_arc_6_3_the_package_ships_stubs_and_py_typed(self):
        package = importlib.resources.files("menura")
        self.assertTrue(package.joinpath("py.typed").is_file())
        self.assertTrue(package.joinpath("__init__.pyi").is_file())
        stubs = package.joinpath("__init__.pyi").read_text(encoding="utf-8")
        for name in ("def validate", "def migrate", "def diff", "class Handle", "class Result"):
            self.assertIn(name, stubs)

    def test_arc_6_3_the_extension_is_built_for_the_stable_abi(self):
        extension = pathlib.Path(menura.menura.__file__).name
        self.assertTrue(extension.endswith((".abi3.so", ".pyd")), extension)


if __name__ == "__main__":
    unittest.main()
