"""Unit tests for soak argument bounds and report schemas."""

from __future__ import annotations

import unittest

from scripts import soak


def memory_report() -> dict[str, object]:
    return {
        "schema_version": 1, "iterations": 10000, "completed_iterations": 10000,
        "baseline_rss_bytes": 100_000_000, "peak_rss_bytes": 101_000_000,
        "growth_bytes": 1_000_000, "growth_limit_bytes": 64 * 1024 * 1024,
        "absolute_limit_bytes": 512 * 1024 * 1024, "pixel_checksum": 1234, "result": "pass",
    }


class SoakReportTests(unittest.TestCase):
    def test_argument_validation_and_default(self) -> None:
        self.assertEqual(soak.parse_args([]).minutes, 5)
        self.assertEqual(soak.parse_args(["--minutes", "1", "--seed", "0"]).seed, 0)
        for args in (("--minutes", "0"), ("--minutes", "21"),
                     ("--seed", "4294967296"), ("--output", "/tmp/soak.json")):
            with self.subTest(args=args), self.assertRaises(SystemExit):
                soak.parse_args(list(args))

    def test_memory_report_schema_and_rss_consistency(self) -> None:
        report = memory_report()
        soak.validate_memory_report(report)
        report["growth_bytes"] = True
        with self.assertRaisesRegex(soak.SoakError, "nonnegative integer"):
            soak.validate_memory_report(report)
        report = memory_report()
        report["growth_bytes"] = 8
        with self.assertRaisesRegex(soak.SoakError, "inconsistent"):
            soak.validate_memory_report(report)

    def test_soak_report_schema_requires_the_expected_bounded_budget(self) -> None:
        names = (
            "release-configure", "release-build", "render-memory-growth", "asan-configure",
            "asan-fuzzer-build", "xml-fuzz", "shape-fuzz", "tsan-configure", "tsan-build",
            "tsan-worker-queue",
        )
        phases = [{"name": name, "status": "skipped", "command": [], "return_code": None,
                   "elapsed_seconds": 0.0, "stdout_log": "not run", "stderr_log": "",
                   "timeout_seconds": 0.0} for name in names]
        report = {
            "schema_version": 1, "kind": "vexel_soak", "status": "failure", "seed": 4,
            "minutes": 2, "fuzz_budget_seconds": 120, "fuzz_campaign_seconds_each": 60,
            "memory_iterations": 10000, "memory_growth": None, "phases": phases,
            "child_reports": {}, "elapsed_seconds": 1.0, "output_path": ".cache/soak/latest.json",
        }
        soak.validate_report(report)
        report["status"] = "pass"
        with self.assertRaisesRegex(soak.SoakError, "status does not match"):
            soak.validate_report(report)
        report["status"] = "failure"
        report["fuzz_budget_seconds"] = 119
        with self.assertRaisesRegex(soak.SoakError, "fuzz budget"):
            soak.validate_report(report)

    def test_shape_campaign_accepts_a_full_duration_below_case_count_floor(self) -> None:
        report = {"result": "pass", "seed": 9, "cases": 961536,
                  "process_cpu_seconds": 30.005, "sanitizer_reports": 0,
                  "crashes": 0, "invariant_failures": 0, "sanitizer_stderr_bytes": 0}
        soak.validate_shape_child(report, 9, 30)
        report["process_cpu_seconds"] = 29.9
        with self.assertRaisesRegex(soak.SoakError, "bounded seeded run"):
            soak.validate_shape_child(report, 9, 30)

    def test_xml_campaign_report_requires_duration_and_no_sanitizer_output(self) -> None:
        report = {"status": "pass", "seed": 9, "iterations": 2000000,
                  "timed_out": False, "sanitizer_output": False, "duration_seconds": 30}
        soak.validate_xml_child(report, 9, 30)
        report["sanitizer_output"] = True
        with self.assertRaisesRegex(soak.SoakError, "clean seeded run"):
            soak.validate_xml_child(report, 9, 30)


if __name__ == "__main__":
    unittest.main()
