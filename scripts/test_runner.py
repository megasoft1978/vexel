#!/usr/bin/env python3
"""Run the repository unittest suite and write stable machine-readable results."""

from __future__ import annotations

import argparse
import json
import sys
import time
import unittest
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCHEMA = ROOT / "docs/schemas/test-results.schema.json"


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat(timespec="milliseconds").replace("+00:00", "Z")


class Result(unittest.TestResult):
    def __init__(self) -> None:
        super().__init__()
        self.tests: list[dict[str, str]] = []

    def startTest(self, test: unittest.case.TestCase) -> None:
        self.current_id = test.id()
        super().startTest(test)

    def addSuccess(self, test: unittest.case.TestCase) -> None:
        self.tests.append({"id": test.id(), "status": "passed"})
        super().addSuccess(test)

    def addFailure(self, test: unittest.case.TestCase, err: object) -> None:
        self.tests.append({"id": test.id(), "status": "failed"})
        super().addFailure(test, err)

    def addError(self, test: unittest.case.TestCase, err: object) -> None:
        self.tests.append({"id": test.id(), "status": "error"})
        super().addError(test, err)

    def addSkip(self, test: unittest.case.TestCase, reason: str) -> None:
        self.tests.append({"id": test.id(), "status": "skipped"})
        super().addSkip(test, reason)

    def addExpectedFailure(self, test: unittest.case.TestCase, err: object) -> None:
        self.tests.append({"id": test.id(), "status": "expected_failure"})
        super().addExpectedFailure(test, err)

    def addUnexpectedSuccess(self, test: unittest.case.TestCase) -> None:
        self.tests.append({"id": test.id(), "status": "unexpected_success"})
        super().addUnexpectedSuccess(test)

    def addSubTest(self, test: unittest.case.TestCase, subtest: unittest.case.TestCase,
                   err: object | None) -> None:
        if err is not None:
            self.tests.append({"id": subtest.id(), "status": "failed"})
        super().addSubTest(test, subtest, err)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args(argv)
    started_at = utc_now()
    started = time.monotonic()
    suite = unittest.defaultTestLoader.discover(str(ROOT / "tests"), pattern="test_*.py", top_level_dir=str(ROOT))
    result = Result()
    unittest.TextTestRunner(stream=sys.stderr, verbosity=2, resultclass=lambda *a, **k: result).run(suite)
    for test, trace in result.failures + result.errors:
        print(f"FAILED {test.id()}\n{trace}", file=sys.stderr)
    ended_at = utc_now()
    statuses = [item["status"] for item in result.tests]
    counts = {key: statuses.count(key) for key in ("passed", "failed", "error", "skipped", "expected_failure", "unexpected_success")}
    payload = {"started_at": started_at, "ended_at": ended_at,
               "elapsed_seconds": round(time.monotonic() - started, 6), "counts": counts,
               "tests": result.tests}
    args.output = args.output if args.output.is_absolute() else ROOT / args.output
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    if not valid(payload, schema):
        print("test results did not match schema", file=sys.stderr)
        return 2
    print(f"Ran {len(result.tests)} tests: {counts['passed']} passed, {counts['failed']} failed, "
          f"{counts['error']} errors, {counts['skipped']} skipped; {payload['elapsed_seconds']:.3f}s")
    return 0 if result.wasSuccessful() and counts["skipped"] == 0 else 1


def valid(value: object, schema: dict[str, object]) -> bool:
    """Small schema checker for the fixed result format, avoiding dependencies."""
    if isinstance(value, dict):
        properties = schema.get("properties", {})
        required = schema.get("required", [])
        if any(key not in value for key in required) or set(value) - set(properties):
            return False
        return all(valid(child, properties[key]) for key, child in value.items())
    if isinstance(value, list):
        return all(valid(item, schema["items"]) for item in value)
    if schema.get("type") == "string":
        return isinstance(value, str) and ("enum" not in schema or value in schema["enum"])
    if schema.get("type") == "integer":
        return type(value) is int and value >= schema.get("minimum", 0)
    if schema.get("type") == "number":
        return type(value) in (int, float) and value >= schema.get("minimum", 0)
    return True


if __name__ == "__main__":
    raise SystemExit(main())
