"""Focused contract tests for repeated local performance violations."""
from __future__ import annotations

import copy
import json
import math
import unittest

from scripts import perf_gate as gate


class PerfGateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.baseline = gate.validate_baseline(gate.load_json(gate.BASELINE_PATH, "baseline"))
        self.metrics = {name: 1.0 for name in gate.METRICS}
        self.limits = {name: 2.0 for name in gate.METRICS}
        self.baselines = {name: 1.0 for name in gate.METRICS}
        self.tolerances = {name: 0.1 for name in gate.METRICS}
        self.history = gate.empty_history()

    def test_matching_baseline_host_and_workloads_are_comparable(self) -> None:
        hardware = dict(self.baseline["host_toolchain"], machine="ignored platform detail")
        cases = {name: {"input_sha256": row["input_sha256"]}
                 for name, row in self.baseline["cases"].items()}
        self.assertEqual([], gate.comparison_reason(self.baseline, hardware, cases))

    def test_host_or_workload_mismatch_is_incomparable(self) -> None:
        hardware = dict(self.baseline["host_toolchain"], os="macOS changed")
        cases = {name: {"input_sha256": row["input_sha256"]}
                 for name, row in self.baseline["cases"].items()}
        cases["100MiB"]["input_sha256"] = "0" * 64
        reasons = gate.comparison_reason(self.baseline, hardware, cases)
        self.assertEqual(2, len(reasons))
        results, unchanged = gate.evaluate(self.metrics, self.limits, self.baselines,
                                           self.tolerances, self.history, False)
        self.assertTrue(all(item["status"] == "incomparable" for item in results.values()))
        self.assertEqual(self.history, unchanged)

    def test_one_violation_warns_and_second_fails(self) -> None:
        values = dict(self.metrics, **{"10MiB.first_preview_p95_ns": 1.2})
        first, history = gate.evaluate(values, self.limits, self.baselines,
                                       self.tolerances, self.history, True)
        self.assertEqual("warning", first["10MiB.first_preview_p95_ns"]["status"])
        second, _ = gate.evaluate(values, self.limits, self.baselines,
                                  self.tolerances, history, True)
        self.assertEqual("fail", second["10MiB.first_preview_p95_ns"]["status"])

    def test_healthy_observation_resets_violation_streak(self) -> None:
        values = dict(self.metrics, **{"10MiB.first_preview_p95_ns": 1.2})
        _, history = gate.evaluate(values, self.limits, self.baselines,
                                   self.tolerances, self.history, True)
        healthy, history = gate.evaluate(self.metrics, self.limits, self.baselines,
                                         self.tolerances, history, True)
        self.assertEqual("pass", healthy["10MiB.first_preview_p95_ns"]["status"])
        self.assertEqual(0, history["metrics"]["10MiB.first_preview_p95_ns"]["streak"])
        again, _ = gate.evaluate(values, self.limits, self.baselines,
                                 self.tolerances, history, True)
        self.assertEqual("warning", again["10MiB.first_preview_p95_ns"]["status"])

    def test_metric_streaks_are_independent(self) -> None:
        values = dict(self.metrics, **{"10MiB.first_preview_p95_ns": 1.2})
        _, history = gate.evaluate(values, self.limits, self.baselines,
                                   self.tolerances, self.history, True)
        values["10MiB.first_preview_p95_ns"] = 1.0
        values["100MiB.max_step_ns"] = 3.0
        results, history = gate.evaluate(values, self.limits, self.baselines,
                                         self.tolerances, history, True)
        self.assertEqual("pass", results["10MiB.first_preview_p95_ns"]["status"])
        self.assertEqual("warning", results["100MiB.max_step_ns"]["status"])
        self.assertEqual(0, history["metrics"]["10MiB.first_preview_p95_ns"]["streak"])
        self.assertEqual(1, history["metrics"]["100MiB.max_step_ns"]["streak"])

    def test_unavailable_metric_does_not_reset_or_advance(self) -> None:
        values = dict(self.metrics, **{"10MiB.first_preview_p95_ns": 1.2})
        _, history = gate.evaluate(values, self.limits, self.baselines,
                                   self.tolerances, self.history, True)
        before = copy.deepcopy(history["metrics"]["10MiB.first_preview_p95_ns"])
        values["10MiB.first_preview_p95_ns"] = None
        result, after = gate.evaluate(values, self.limits, self.baselines,
                                      self.tolerances, history, True)
        self.assertEqual("unavailable", result["10MiB.first_preview_p95_ns"]["status"])
        self.assertEqual(before, after["metrics"]["10MiB.first_preview_p95_ns"])

    def test_noisy_metric_does_not_advance_violation_streak(self) -> None:
        metric = "10MiB.first_preview_p95_ns"
        values = dict(self.metrics, **{metric: 1.2})
        noisy, history = gate.evaluate(values, self.limits, self.baselines,
                                       self.tolerances, self.history, True, {metric})
        self.assertEqual("noisy", noisy[metric]["status"])
        self.assertEqual(0, history["metrics"][metric]["streak"])
        stable, history = gate.evaluate(values, self.limits, self.baselines,
                                        self.tolerances, history, True)
        self.assertEqual("warning", stable[metric]["status"])
        self.assertEqual(1, history["metrics"][metric]["streak"])

    def test_malformed_history_and_values_fail_closed(self) -> None:
        extra = gate.empty_history()
        extra["unknown"] = 1
        with self.assertRaises(gate.GateError):
            gate.validate_history(extra)
        invalid_streak = gate.empty_history()
        invalid_streak["metrics"]["100MiB.max_step_ns"]["streak"] = 3
        invalid_streak["metrics"]["100MiB.max_step_ns"]["last_status"] = "violation"
        with self.assertRaises(gate.GateError):
            gate.validate_history(invalid_streak)
        with self.assertRaises(gate.GateError):
            gate.evaluate(dict(self.metrics, **{"100MiB.max_step_ns": -1}), self.limits,
                          self.baselines, self.tolerances, self.history, True)
        with self.assertRaises(gate.GateError):
            gate.evaluate(dict(self.metrics, **{"100MiB.max_step_ns": math.nan}), self.limits,
                          self.baselines, self.tolerances, self.history, True)
        with self.assertRaises(gate.GateError):
            gate.evaluate(dict(self.metrics, **{"100MiB.max_step_ns": 10 ** 1000}), self.limits,
                          self.baselines, self.tolerances, self.history, True)
        malformed_baseline = copy.deepcopy(self.baseline)
        malformed_baseline["unexpected"] = 1
        with self.assertRaises(gate.GateError):
            gate.validate_baseline(malformed_baseline)

    def test_deterministic_serialization_and_stretch_targets_are_report_only(self) -> None:
        results, _ = gate.evaluate(self.metrics, self.limits, self.baselines,
                                   self.tolerances, self.history, True)
        document = {"metrics": results, "stretch_targets": self.baseline["stretch_targets"]}
        encoded = json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n"
        self.assertEqual(encoded, json.dumps(document, indent=2, sort_keys=True, allow_nan=False) + "\n")
        self.assertEqual("pass", results["100MiB.peak_rss_bytes"]["status"])
        self.assertEqual(25 * 1024 * 1024, self.baseline["stretch_targets"]["app_bundle_bytes"])


if __name__ == "__main__":
    unittest.main()
