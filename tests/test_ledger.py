"""Tests for transition validation and concurrent ledger appends."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

from scripts.ledger import LedgerError, ROOT, append_entry, validate_bytes

LEDGER_TOOL = ROOT / "scripts" / "ledger.py"
BASE_SHA = "b90dfbc44623e9834852b19513116149c83a15f0"
HEAD_SHA = "beb0cdd"


def entry(wo_id: str, event: str, run_id: str, status: str) -> dict[str, object]:
    return {
        "ts": "2026-10-08T13:00:00Z",
        "wo_id": wo_id,
        "event": event,
        "revision": 1,
        "base_sha": BASE_SHA,
        "head_sha": HEAD_SHA,
        "run_id": run_id,
        "session_id": "test-session",
        "model": "gpt-6-luna",
        "cli_version": "codex-cli test",
        "duration_s": 0.0,
        "usage": {"input_tokens": None, "output_tokens": None},
        "status": status,
        "artifacts": [],
        "note": "test",
    }


class LedgerTests(unittest.TestCase):
    def setUp(self) -> None:
        cache = ROOT / ".cache"
        cache.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(dir=cache)
        self.directory = Path(self.temp.name)
        self.path = self.directory / "ledger.jsonl"

    def tearDown(self) -> None:
        self.temp.cleanup()

    def test_seeded_history_is_valid(self) -> None:
        data = (ROOT / "ledger.jsonl").read_bytes()
        entries = validate_bytes(data)
        self.assertTrue(entries)
        boot01 = [item for item in entries if item["wo_id"] == "WO-BOOT-01"]
        self.assertEqual(boot01[-1]["event"], "merged")

    def test_interleaved_work_orders_keep_independent_transitions(self) -> None:
        entries = [
            entry("WO-ONE", "ready", "r1", "ready"),
            entry("WO-ONE", "run_started", "r2", "running"),
            entry("WO-ONE", "run_finished", "r3", "complete"),
            entry("WO-ONE", "verified", "r4", "pass"),
            entry("WO-TWO", "ready", "r5", "ready"),
            entry("WO-ONE", "accepted", "r6", "accepted"),
            entry("WO-ONE", "merged", "r7", "merged"),
        ]
        data = "".join(json.dumps(item) + "\n" for item in entries).encode()
        self.assertEqual(len(validate_bytes(data)), len(entries))

    def test_malformed_json_and_unknown_fields_fail(self) -> None:
        with self.assertRaises(LedgerError):
            validate_bytes(b"{broken}\n")
        bad = entry("WO-TEST-01", "ready", "r1", "ready")
        bad["unexpected"] = True
        with self.assertRaises(LedgerError):
            validate_bytes((json.dumps(bad) + "\n").encode())
        bad = entry("WO-TEST-01", "ready", "r2", "ready")
        bad["event"] = []
        with self.assertRaises(LedgerError):
            validate_bytes((json.dumps(bad) + "\n").encode())

    def test_duplicate_run_id_fails(self) -> None:
        first = entry("WO-TEST-01", "ready", "same-run", "ready")
        second = entry("WO-TEST-02", "ready", "same-run", "ready")
        data = (json.dumps(first) + "\n" + json.dumps(second) + "\n").encode()
        with self.assertRaisesRegex(LedgerError, "duplicate run_id"):
            validate_bytes(data)

    def test_invalid_transition_fails(self) -> None:
        bad = entry("WO-TEST-01", "merged", "r1", "merged")
        with self.assertRaisesRegex(LedgerError, "invalid transition"):
            validate_bytes((json.dumps(bad) + "\n").encode())

    def test_invalid_append_does_not_change_ledger(self) -> None:
        ready = entry("WO-TEST-01", "ready", "r1", "ready")
        append_entry(self.path, ready)
        before = self.path.read_bytes()
        invalid = entry("WO-TEST-01", "merged", "r2", "merged")
        with self.assertRaises(LedgerError):
            append_entry(self.path, invalid)
        self.assertEqual(self.path.read_bytes(), before)

    def test_cli_rejects_invalid_entry_without_mutation(self) -> None:
        self.path.write_bytes(b"")
        invalid = entry("WO-TEST-01", "merged", "bad-transition", "merged")
        before = self.path.read_bytes()
        result = subprocess.run(
            [sys.executable, str(LEDGER_TOOL), "append", str(self.path),
             "--entry-json", json.dumps(invalid)],
            capture_output=True, text=True,
        )
        self.assertEqual(result.returncode, 2)
        self.assertIn("invalid transition", result.stderr)
        self.assertEqual(self.path.read_bytes(), before)

    def test_concurrent_appends_remain_complete_and_valid(self) -> None:
        def append(index: int) -> None:
            item = entry(f"WO-CONCURRENT-{index:02d}", "ready", f"run-{index}", "ready")
            result = subprocess.run(
                [sys.executable, str(LEDGER_TOOL), "append", str(self.path),
                 "--entry-json", json.dumps(item)],
                capture_output=True, text=True,
            )
            if result.returncode:
                raise AssertionError(result.stderr)

        with ThreadPoolExecutor(max_workers=8) as pool:
            list(pool.map(append, range(24)))
        entries = validate_bytes(self.path.read_bytes())
        self.assertEqual(len(entries), 24)
        self.assertEqual(len({item["run_id"] for item in entries}), 24)


if __name__ == "__main__":
    unittest.main()
