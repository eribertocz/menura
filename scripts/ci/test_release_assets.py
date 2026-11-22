"""Unit tests of the release assets check (CON-0.3). Run: python3 -m unittest discover scripts/ci"""
import pathlib
import sys
import tempfile
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import release_assets  # noqa: E402

ROOT = pathlib.Path(__file__).resolve().parents[2]


class FakeRelease:
    """A release in memory, with the operations of `release_assets.ReleaseApi`."""

    def __init__(self, assets=None, dropped=()):
        self.assets = dict(assets or {})
        self.dropped = set(dropped)
        self.next_identifier = 1000
        self.calls = []

    def list_assets(self):
        self.calls.append(("list",))
        return [{"id": identifier, "name": name} for name, (identifier, _) in sorted(self.assets.items())]

    def delete_asset(self, identifier):
        self.calls.append(("delete", identifier))
        self.assets = {name: entry for name, entry in self.assets.items() if entry[0] != identifier}

    def upload_asset(self, name, content):
        self.calls.append(("upload", name))
        if name in self.dropped:
            return
        self.next_identifier += 1
        self.assets[name] = (self.next_identifier, content)


def write_schemas(directory, names):
    schemas = pathlib.Path(directory) / release_assets.SCHEMA_DIRECTORY
    schemas.mkdir(parents=True)
    for name in names:
        (schemas / name).write_text(f'{{"title": "{name}"}}\n', encoding="utf-8")
    (schemas / "README.md").write_text("not a schema\n", encoding="utf-8")


class ReleaseAssetsTest(unittest.TestCase):
    """CON-0.3, CI-2 `release-assets`."""

    def test_con_0_3_finds_every_schema_of_the_tree_in_name_order(self):
        with tempfile.TemporaryDirectory() as directory:
            write_schemas(directory, ["query.schema.json", "audit-record.schema.json"])
            names = [path.name for path in release_assets.schema_files(pathlib.Path(directory))]
        self.assertEqual(names, ["audit-record.schema.json", "query.schema.json"])

    def test_con_0_3_the_repository_has_the_twelve_contract_schemas(self):
        self.assertEqual(len(release_assets.schema_files(ROOT)), 12)

    def test_con_0_3_attaches_every_schema(self):
        release = FakeRelease()
        schemas = {"a.schema.json": b"a", "b.schema.json": b"b"}
        release_assets.attach(release, schemas)
        self.assertEqual(release.assets["a.schema.json"][1], b"a")
        self.assertEqual(release.assets["b.schema.json"][1], b"b")
        self.assertNotIn("delete", [call[0] for call in release.calls])

    def test_con_0_3_replaces_an_asset_of_the_same_name(self):
        release = FakeRelease({"a.schema.json": (7, b"old"), "notes.txt": (8, b"keep")})
        release_assets.attach(release, {"a.schema.json": b"new"})
        self.assertEqual(release.calls, [("list",), ("delete", 7), ("upload", "a.schema.json")])
        self.assertEqual(release.assets["a.schema.json"][1], b"new")
        self.assertEqual(release.assets["notes.txt"], (8, b"keep"))

    def test_con_0_3_nothing_is_missing_when_every_schema_is_attached(self):
        release = FakeRelease()
        schemas = {"a.schema.json": b"a", "b.schema.json": b"b"}
        release_assets.attach(release, schemas)
        self.assertEqual(release_assets.missing(release, schemas), [])

    def test_con_0_3_fails_when_a_schema_is_missing(self):
        release = FakeRelease(dropped={"b.schema.json"})
        schemas = {"a.schema.json": b"a", "b.schema.json": b"b"}
        release_assets.attach(release, schemas)
        self.assertEqual(release_assets.missing(release, schemas), ["b.schema.json"])

    def test_con_0_3_run_fails_when_a_schema_is_missing(self):
        with tempfile.TemporaryDirectory() as directory:
            write_schemas(directory, ["a.schema.json", "b.schema.json"])
            status = release_assets.run(FakeRelease(dropped={"a.schema.json"}), pathlib.Path(directory))
        self.assertEqual(status, 1)

    def test_con_0_3_run_succeeds_when_every_schema_is_attached(self):
        with tempfile.TemporaryDirectory() as directory:
            write_schemas(directory, ["a.schema.json", "b.schema.json"])
            release = FakeRelease({"a.schema.json": (3, b"old")})
            status = release_assets.run(release, pathlib.Path(directory))
        self.assertEqual(status, 0)
        self.assertEqual(sorted(release.assets), ["a.schema.json", "b.schema.json"])

    def test_con_0_3_run_fails_when_the_tag_has_no_schema(self):
        with tempfile.TemporaryDirectory() as directory:
            write_schemas(directory, [])
            self.assertEqual(release_assets.run(FakeRelease(), pathlib.Path(directory)), 1)

    def test_ci_2_requires_the_repository_the_token_and_the_tag(self):
        environment = {"GITHUB_REPOSITORY": "owner/name", "GITHUB_TOKEN": "token", "RELEASE_TAG": ""}
        self.assertEqual(release_assets.main([str(ROOT)], environment), 1)


if __name__ == "__main__":
    unittest.main()
