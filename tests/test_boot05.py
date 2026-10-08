"""Focused contracts for the BOOT-05 runner, sanitizer, and fuzz foundation."""

from __future__ import annotations

import contextlib
import gzip
import io
import json
import random
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import check_fuzz_workflow
import fuzz_smoke
import test_runner
import wo_driver


class Boot05Contracts(unittest.TestCase):
    def test_retained_sanitizer_probe_traces_are_compressed_evidence(self) -> None:
        evidence = ROOT / "artifacts/WO-BOOT-05/probes"
        report = json.loads((evidence / "probes.json").read_text())
        self.assertTrue(report["all_detected"])
        for item in report["probes"].values():
            trace = evidence / item["stderr_artifact"]
            self.assertTrue(trace.is_file())
            self.assertTrue(gzip.decompress(trace.read_bytes()))

    def test_test_results_schema_accepts_machine_result_shape(self) -> None:
        schema = json.loads((ROOT / "docs/schemas/test-results.schema.json").read_text())
        item = {"started_at": "2026-10-08T00:00:00Z", "ended_at": "2026-10-08T00:00:01Z",
                "elapsed_seconds": 1.0,
                "counts": {key: 0 for key in ("passed", "failed", "error", "skipped", "expected_failure", "unexpected_success")},
                "tests": []}
        self.assertTrue(test_runner.valid(item, schema))
        self.assertFalse(test_runner.valid(item | {"unexpected": True}, schema))

    def test_strict_schema_matches_frozen_acceptance_set(self) -> None:
        body = (ROOT / "work/WO-BOOT-05.md").read_text()
        ids = wo_driver.acceptance_ids(body)
        schema = json.loads((ROOT / "docs/schemas/executor-report.schema.json").read_text())
        strict = wo_driver.strict_executor_schema(schema, ids)
        acceptance = strict["properties"]["acceptance"]
        self.assertEqual(set(acceptance["required"]), ids)
        self.assertEqual(set(acceptance["properties"]), ids)
        self.assertIs(acceptance["additionalProperties"], False)
        self.assertNotIn("minProperties", json.dumps(schema))

    def test_fuzz_cli_bounds_and_deterministic_printable_mutation(self) -> None:
        first = fuzz_smoke.mutation(random.Random(12648430), 5)
        second = fuzz_smoke.mutation(random.Random(12648430), 5)
        self.assertEqual(first, second)
        self.assertTrue(all("\x00" not in argument and len(argument) <= 96 for argument in first))
        self.assertTrue(fuzz_smoke.expected_success(["--help"], "usage: svgworkbench --help\n"))
        self.assertFalse(fuzz_smoke.expected_success(["--help", "extra"], "usage: svgworkbench\n"))
        self.assertTrue(fuzz_smoke.expected_success(["--version"], "svgworkbench 0.0.1\n"))
        self.assertEqual(fuzz_smoke.iteration_budget(3), 192)
        self.assertEqual(fuzz_smoke.iteration_budget(60), 3840)
        with contextlib.redirect_stderr(io.StringIO()):
            with self.assertRaises(SystemExit):
                fuzz_smoke.parse_args(["--duration", "61"])

    def test_workflow_policy_rejects_non_macos_runner(self) -> None:
        workflow = (ROOT / ".github/workflows/fuzz.yml").read_text()
        check_fuzz_workflow.check(workflow)
        with self.assertRaisesRegex(ValueError, "macos-15"):
            check_fuzz_workflow.check(workflow.replace("runs-on: macos-15", "runs-on: ubuntu-latest"))


if __name__ == "__main__":
    unittest.main()
