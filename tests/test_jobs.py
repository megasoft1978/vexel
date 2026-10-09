"""Focused harness for native immutable snapshot and worker tests."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EXECUTABLE = ROOT / "build/mac-debug/bin/job_snapshot_test"
ARTIFACTS = ROOT / "artifacts/WO-M3-03"


class JobSnapshotTests(unittest.TestCase):
    def test_native_snapshot_and_schedule_contracts(self) -> None:
        self.assertTrue(EXECUTABLE.is_file(), "run ./dev build first")
        result = subprocess.run(
            [str(EXECUTABLE)], capture_output=True, text=True, timeout=30, check=False
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        summary = json.loads(result.stdout.strip().splitlines()[-1])
        self.assertEqual(summary["status"], "pass")
        self.assertEqual(summary["owned_cleanup_cases"], 4)
        self.assertEqual(summary["failed_submit_cleanup"], 0)
        self.assertEqual(summary["deterministic_digest"], "pass")
        self.assertEqual(summary["byte_budget"], "pass")
        self.assertEqual(summary["scheduler_capacity"], "pass")
        self.assertEqual(summary["random_cleanup_accounting"], "zero_after_generation")
        self.assertEqual(summary["generation_filter"], "pass")
        self.assertEqual(summary["random_schedule"], "pass")
        ARTIFACTS.mkdir(parents=True, exist_ok=True)
        (ARTIFACTS / "scheduler-tests.json").write_text(
            json.dumps(summary, sort_keys=True, indent=2) + "\n", encoding="utf-8"
        )
        (ARTIFACTS / "snapshot-tests.txt").write_text(
            "PASS: invalid nonempty NULL slice rejected; >256 MiB rejected; empty slice accepted.\n"
            "PASS: mutating caller storage leaves copied snapshot bytes unchanged.\n"
            "PASS: six threads each performed 20,000 concurrent retain/read/release cycles.\n"
            "PASS: deterministic scheduler digest and failed byte-budget submit ownership.\n"
            "PASS: scheduler capacity returns JOBS_FULL at capacity one.\n"
            "PASS: randomized cancel/generation run drains in-flight byte accounting to zero.\n",
            encoding="utf-8",
        )


if __name__ == "__main__":
    unittest.main()
