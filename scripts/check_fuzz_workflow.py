#!/usr/bin/env python3
"""Enforce the macOS-only bounded XML fuzz workflow contract."""

from __future__ import annotations

import re
import sys
from pathlib import Path


def check(text: str) -> None:
    runners = re.findall(r"^\s*runs-on:\s*(.*?)\s*$", text, re.MULTILINE)
    if runners != ["macos-15"]:
        raise ValueError("fuzz workflow must use only the macos-15 Apple Silicon runner")
    if "pull_request:" not in text or "workflow_dispatch:" not in text:
        raise ValueError("workflow must include pull_request and workflow_dispatch")
    if not re.search(r"^\s{2}schedule:\s*\n\s{4}-\s+cron:", text, re.MULTILINE):
        raise ValueError("workflow must include a scheduled trigger")
    if "./dev fuzz-xml --duration 30 --seed 20261009" not in text:
        raise ValueError("PR/manual smoke must request 30 seconds with an explicit seed")
    if "./dev fuzz-xml --duration 600 --seed 20261009" not in text:
        raise ValueError("scheduled campaign must request 600 seconds with an explicit seed")
    permissions = re.search(r"^permissions:\s*\n((?:^[ \t]+[^\n]+\n)+)", text, re.MULTILINE)
    permission_entries = [] if permissions is None else [
        line.strip() for line in permissions.group(1).splitlines() if line.strip()
    ]
    if permission_entries != ["contents: read"]:
        raise ValueError("workflow must grant only contents: read")
    timeout = re.search(r"^\s{4}timeout-minutes:\s*(\d+)\s*$", text, re.MULTILINE)
    if timeout is None or not 1 <= int(timeout.group(1)) <= 20:
        raise ValueError("workflow job requires a timeout between 1 and 20 minutes")
    actions = re.findall(r"^\s*uses:\s*([^\s#]+)", text, re.MULTILINE)
    if any(not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}", action) for action in actions):
        raise ValueError("every action must be pinned to a full commit SHA")
    if re.search(r"\$\{\{\s*secrets\.|\bsecrets\.[A-Za-z_]", text, re.IGNORECASE):
        raise ValueError("workflow must not use secrets")
    if "upload-artifact" in text:
        paths = re.findall(r"^\s*path:\s*(.*?)\s*$", text, re.MULTILINE)
        if not paths or any(not path.startswith("artifacts/WO-M2-03/failures") for path in paths):
            raise ValueError("failure artifact paths must be confined to task failure evidence")
    logged_paths = re.findall(r"Path\(['\"]([^'\"]+)['\"]\)", text)
    if not logged_paths or any(not path.startswith("artifacts/WO-M2-03/") for path in logged_paths):
        raise ValueError("failure log paths must stay in the task evidence directory")
    if "failure()" not in text and "if: always()" not in text:
        raise ValueError("workflow must retain failure evidence in logs")


def main() -> int:
    try:
        check(Path(".github/workflows/fuzz.yml").read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        print(f"fuzz workflow policy error: {exc}", file=sys.stderr)
        return 1
    print("macOS-only bounded XML fuzz workflow: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
