"""Unit tests of the agent smoke test. Run: python3 -m unittest discover -s scripts/ci -p 'test_agent_smoke.py'"""
import copy
import json
import pathlib
import sys
import unittest

sys.path.insert(0, str(pathlib.Path(__file__).resolve().parent))

import agent_smoke  # noqa: E402


def text_result(document, is_error):
    """A `tools/call` result with one text item holding a JSON document."""
    return {"content": [{"type": "text", "text": json.dumps(document, indent=2)}], "isError": is_error}


def name_not_found(reference, alternatives):
    """A compile error with one `E_NAME_NOT_FOUND` diagnostic on the first metric."""
    error = {
        "code": "E_NAME_NOT_FOUND",
        "message": f"`{reference}` does not name a field or metric of the model; check the name and how it is written",
        "location": {"document": "query", "pointer": "/query/metrics/0"},
    }
    if alternatives:
        error["alternatives"] = alternatives
    return {"contract": "menura.compile_error", "contract_version": "1.2.0", "errors": [error], "warnings": []}


def canned_responses():
    """The responses of a server that meets every expectation, keyed by `id`."""
    results = {
        agent_smoke.INITIALIZE: {
            "capabilities": {"tools": {"listChanged": False}},
            "protocolVersion": "2025-06-18",
            "serverInfo": {"name": "menura-mcp", "version": "0.9.0"},
        },
        agent_smoke.TOOLS_LIST: {"tools": [{"name": name} for name in reversed(agent_smoke.TOOLS)]},
        agent_smoke.DESCRIBE: text_result({"contract": "menura.model_description"}, False),
        agent_smoke.COMPILE_VALID: text_result({"contract": "menura.compiled_plan"}, False),
        agent_smoke.COMPILE_MISSPELLED: text_result(name_not_found("total_amont", ["metric:TOTAL_AMOUNT"]), True),
        agent_smoke.COMPILE_HIDDEN: text_result(name_not_found("total_target", []), True),
        agent_smoke.COMPILE_MISSING: text_result(name_not_found("total_targex", []), True),
    }
    return {identifier: {"jsonrpc": "2.0", "id": identifier, "result": result} for identifier, result in results.items()}


def server_output(responses):
    """The standard output of a server that wrote the given responses, one per line."""
    return "".join(json.dumps(response) + "\n" for response in responses.values())


class AgentSmokeTest(unittest.TestCase):
    """ARC-6.7, PLN-5.4, PLN-5.4a."""

    def failures_after(self, change):
        responses = copy.deepcopy(canned_responses())
        change(responses)
        return agent_smoke.failures(agent_smoke.responses_by_identifier(server_output(responses)))

    def test_arc_6_7_a_server_that_meets_every_expectation_passes(self):
        self.assertEqual(self.failures_after(lambda responses: None), [])

    def test_arc_6_7_the_sequence_is_fixed(self):
        documents = {"model": "m", "catalog_schema": "c", "security_context": "s"}
        messages = agent_smoke.requests(documents)
        self.assertEqual(
            [message.get("method") for message in messages],
            ["initialize", "notifications/initialized", "tools/list"] + ["tools/call"] * 5,
        )
        metrics = [
            json.loads(message["params"]["arguments"]["query"])["query"]["metrics"][0]["metric"]
            for message in messages[4:]
        ]
        self.assertEqual(metrics, ["total_amount", "total_amont", "total_target", "total_targex"])
        self.assertNotIn("id", messages[1])

    def test_arc_6_7_a_missing_tool_fails_step_one(self):
        def drop_tool(responses):
            responses[agent_smoke.TOOLS_LIST]["result"]["tools"].pop()

        lines = self.failures_after(drop_tool)
        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("tools/list: "))

    def test_arc_6_7_a_missing_response_fails_its_step(self):
        lines = self.failures_after(lambda responses: responses.pop(agent_smoke.DESCRIBE))
        self.assertEqual(lines, ["describe: no response"])

    def test_arc_6_7_a_json_rpc_error_fails_its_step(self):
        def to_error(responses):
            responses[agent_smoke.COMPILE_VALID] = {
                "jsonrpc": "2.0",
                "id": agent_smoke.COMPILE_VALID,
                "error": {"code": -32602, "message": "x"},
            }

        lines = self.failures_after(to_error)
        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("compile (valid query): JSON-RPC error"))

    def test_arc_6_7_one_line_per_failed_step(self):
        def break_two(responses):
            responses[agent_smoke.INITIALIZE]["result"]["protocolVersion"] = "2024-11-05"
            responses[agent_smoke.COMPILE_VALID]["result"]["isError"] = True

        lines = self.failures_after(break_two)
        self.assertEqual(len(lines), 2)
        self.assertTrue(lines[0].startswith("initialize: "))
        self.assertTrue(lines[1].startswith("compile (valid query): "))

    def test_pln_5_4_a_misspelled_metric_without_alternatives_fails(self):
        def drop_alternatives(responses):
            responses[agent_smoke.COMPILE_MISSPELLED]["result"] = text_result(name_not_found("total_amont", []), True)

        lines = self.failures_after(drop_alternatives)
        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("compile (misspelled metric): alternatives"))

    def test_pln_5_4_a_misspelled_metric_that_compiles_fails(self):
        def compiles(responses):
            responses[agent_smoke.COMPILE_MISSPELLED]["result"] = text_result({"contract": "menura.compiled_plan"}, False)

        lines = self.failures_after(compiles)
        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("compile (misspelled metric): isError"))

    def test_pln_5_4a_a_hidden_metric_with_another_error_fails(self):
        def reveal(responses):
            document = name_not_found("total_target", [])
            document["errors"][0]["code"] = "E_ACCESS_DENIED"
            responses[agent_smoke.COMPILE_HIDDEN]["result"] = text_result(document, True)

        lines = self.failures_after(reveal)
        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("compile (hidden metric): code"))

    def test_pln_5_4a_a_hidden_metric_with_other_alternatives_fails(self):
        def leak(responses):
            document = name_not_found("total_target", ["metric:TOTAL_TARGET"])
            responses[agent_smoke.COMPILE_HIDDEN]["result"] = text_result(document, True)

        lines = self.failures_after(leak)
        self.assertEqual(len(lines), 1)
        self.assertTrue(lines[0].startswith("compile (hidden metric): "))
        self.assertIn("differs from the missing metric's", lines[0])

    def test_arc_6_7_lines_that_are_not_json_are_ignored(self):
        output = "not json\n\n" + server_output(canned_responses())
        self.assertEqual(agent_smoke.failures(agent_smoke.responses_by_identifier(output)), [])

    def test_arc_6_7_a_binary_that_cannot_run_fails(self):
        self.assertEqual(agent_smoke.main(["/nonexistent/menura-mcp"]), 1)


if __name__ == "__main__":
    unittest.main()
