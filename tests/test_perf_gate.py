"""Focused contract tests for repeated local performance violations."""
from __future__ import annotations

import copy
import json
import math
import unittest

from scripts import perf_gate as gate


def valid_raw_report() -> dict[str, object]:
    trial = {
        "source_bytes": 10, "read_ns": 1, "first_preview_ns": 2,
        "complete_index_ns": 3, "max_step_ns": 1,
        "preview_before_index_complete": True,
        "progress_10_50_90_complete": True, "element_count": 2,
        "index_bytes": 8, "peak_rss_bytes": 16, "process_elapsed_ns": 4,
    }
    case = {
        "source_bytes": 10, "source_sha256": "a" * 64,
        "workload": "test workload", "cold_trials": [copy.deepcopy(trial) for _ in range(5)],
        "warm_trials": [copy.deepcopy(trial) for _ in range(10)],
        "source_read_median_ns": 1, "source_read_p95_ns": 1,
        "first_preview_median_ns": 2, "first_preview_p95_ns": 2,
        "complete_index_median_ns": 3, "max_step_latency_ns": 1,
        "progressive_replay": {"events": [10, 50, 90, 100],
                               "first_preview_before_index_complete": True,
                               "worker_render_completed": True},
        "peak_child_rss_bytes": 16, "index_bytes": 8,
    }
    return {
        "schema_version": 1, "seed": 20261009,
        "machine": "macOS-test", "machine_model": "test model",
        "chip": "test chip", "memory_bytes": 1024,
        "architecture": "arm64", "os": "macOS test",
        "compiler": "clang test", "build": "mac-release",
        "cache_caveat": "run order only",
        "cases": {"10MiB": copy.deepcopy(case), "100MiB": copy.deepcopy(case)},
    }


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

    def test_raw_and_generated_report_reject_nested_unknowns_and_bool_versions(self) -> None:
        raw = valid_raw_report()
        gate.validate_raw(raw)
        malformed_raw = copy.deepcopy(raw)
        malformed_raw["schema_version"] = True
        with self.assertRaises(gate.GateError):
            gate.validate_raw(malformed_raw)
        malformed_raw = copy.deepcopy(raw)
        malformed_raw["cases"]["10MiB"]["progressive_replay"]["surprise"] = 1
        with self.assertRaises(gate.GateError):
            gate.validate_raw(malformed_raw)

        result_path = gate.ROOT / "artifacts/WO-BOOT-08/perf-gate.json"
        result = gate.load_json(result_path, "test result report")
        gate.validate_result(result)
        improved = copy.deepcopy(result)
        metric = improved["gate"]["metrics"]["100MiB.first_preview_p95_ns"]
        metric["delta_from_baseline"] = -1
        gate.validate_result(improved)
        invalid_measurement = copy.deepcopy(improved)
        invalid_measurement["gate"]["metrics"]["100MiB.first_preview_p95_ns"]["value"] = -1
        with self.assertRaises(gate.GateError):
            gate.validate_result(invalid_measurement)
        malformed_result = copy.deepcopy(result)
        malformed_result["schema_version"] = True
        with self.assertRaises(gate.GateError):
            gate.validate_result(malformed_result)
        malformed_result = copy.deepcopy(result)
        malformed_result["cases"]["100MiB"]["summary"]["surprise"] = 1
        with self.assertRaises(gate.GateError):
            gate.validate_result(malformed_result)

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
