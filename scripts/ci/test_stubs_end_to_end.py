"""Unit tests of the stub modules. Run: python3 -m unittest discover -s scripts/ci -p 'test_stubs_end_to_end.py'"""
import copy
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import stubs_end_to_end  # noqa: E402
from stubs_end_to_end import StubError, sha256_of  # noqa: E402

FINGERPRINT = "3" * 64


def valid_plan():
    """A compiled plan whose cache key is consistent with its members."""
    plan = {
        "contract": "menura.compiled_plan",
        "dialect": "duckdb",
        "data_source": "stub:catalog",
        "sql": "SELECT \"status\" FROM \"t\" WHERE \"status\" = CAST($1 AS VARCHAR)",
        "parameters": [{"position": 1, "logical_type": {"kind": "text"}, "value": "OPEN"}],
        "result_schema": [
            {"name": "status", "role": "dimension", "logical_type": {"kind": "text"},
             "physical_type": "VARCHAR", "nullable": False},
        ],
    }
    return with_cache_key(plan)


def with_cache_key(plan):
    """`plan` with the cache key PLN-3 and PLN-3.1 define for its members."""
    hashed = {member: plan[member] for member in ("dialect", "data_source", "sql", "parameters", "result_schema")}
    plan_hash = sha256_of(hashed)
    keyed = {"plan_hash": plan_hash, "security_fingerprint": FINGERPRINT, "data_versions": None}
    plan["cache_key"] = {**keyed, "key": sha256_of(keyed)}
    return plan


DESCRIPTION = {"contract": "menura.model_description", "security_fingerprint": FINGERPRINT}


class ExecutorStubTest(unittest.TestCase):
    def test_accepts_a_valid_plan(self):
        parameters, columns = stubs_end_to_end.check_plan(valid_plan(), DESCRIPTION)
        self.assertEqual([parameter["value"] for parameter in parameters], ["OPEN"])
        self.assertEqual([column["name"] for column in columns], ["status"])

    def test_refuses_a_plan_missing_parameters(self):
        plan = valid_plan()
        del plan["parameters"]
        with self.assertRaisesRegex(StubError, "no `parameters`"):
            stubs_end_to_end.check_plan(plan, DESCRIPTION)

    def test_refuses_parameters_out_of_order(self):
        plan = valid_plan()
        plan["parameters"][0]["position"] = 2
        with self.assertRaisesRegex(StubError, "out of order"):
            stubs_end_to_end.check_plan(with_cache_key(plan), DESCRIPTION)

    def test_refuses_an_untyped_parameter(self):
        plan = valid_plan()
        del plan["parameters"][0]["logical_type"]
        with self.assertRaisesRegex(StubError, "no logical type"):
            stubs_end_to_end.check_plan(with_cache_key(plan), DESCRIPTION)

    def test_refuses_placeholders_without_parameters(self):
        plan = valid_plan()
        plan["parameters"] = []
        with self.assertRaisesRegex(StubError, "placeholders"):
            stubs_end_to_end.check_plan(with_cache_key(plan), DESCRIPTION)

    def test_refuses_a_result_schema_with_a_repeated_name(self):
        plan = valid_plan()
        plan["result_schema"].append(copy.deepcopy(plan["result_schema"][0]))
        with self.assertRaisesRegex(StubError, "every column once"):
            stubs_end_to_end.check_plan(with_cache_key(plan), DESCRIPTION)

    def test_refuses_a_stale_plan_hash(self):
        plan = valid_plan()
        plan["sql"] += " "
        with self.assertRaisesRegex(StubError, "plan_hash"):
            stubs_end_to_end.check_plan(plan, DESCRIPTION)

    def test_refuses_a_description_of_another_context(self):
        description = {**DESCRIPTION, "security_fingerprint": "4" * 64}
        with self.assertRaisesRegex(StubError, "different security contexts"):
            stubs_end_to_end.check_plan(valid_plan(), description)


class CanonicalJsonTest(unittest.TestCase):
    def test_sorts_members_and_drops_whitespace(self):
        self.assertEqual(stubs_end_to_end.canonical_json({"b": [1, None], "a": "é"}), '{"a":"é","b":[1,null]}')


class CatalogStubTest(unittest.TestCase):
    def test_the_state_names_the_schema_and_its_relations(self):
        schema = stubs_end_to_end.catalog_schema()
        state = stubs_end_to_end.catalog_state()
        self.assertEqual(state["schema_document_id"], schema["document_id"])
        self.assertEqual([relation["source"] for relation in state["relations"]],
                         [relation["source"] for relation in schema["relations"]])


if __name__ == "__main__":
    unittest.main()
