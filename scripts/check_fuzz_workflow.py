#!/usr/bin/env python3
"""Enforce the macOS-only bounded fuzz workflow contract."""

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
    if "./dev fuzz --duration 3" not in text:
        raise ValueError("PR/manual smoke must run ./dev fuzz --duration 3")
    if "./dev fuzz --duration 60" not in text:
        raise ValueError("scheduled smoke must run ./dev fuzz --duration 60")
    if not re.search(r"timeout-minutes:\s*(?:[1-9]|[1-5][0-9]|60)\s*$", text, re.MULTILINE):
        raise ValueError("workflow job requires a bounded timeout of at most 60 minutes")
    actions = re.findall(r"^\s*uses:\s*([^\s#]+)", text, re.MULTILINE)
    if not actions or any(not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}", action) for action in actions):
        raise ValueError("every action must be pinned to a full commit SHA")
    if re.search(r"\$\{\{\s*secrets\.", text, re.IGNORECASE):
        raise ValueError("workflow must not use secrets")
    if "upload-artifact" in text and re.search(r"path:\s*(?:\.|\$\{\{\s*github\.workspace", text):
        raise ValueError("workflow must not upload repository or user data")


def main() -> int:
    path = Path(".github/workflows/fuzz.yml")
    try:
        check(path.read_text(encoding="utf-8"))
    except (OSError, ValueError) as exc:
        print(f"fuzz workflow policy error: {exc}", file=sys.stderr)
        return 1
    print("macOS-only bounded fuzz workflow: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
