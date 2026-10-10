#!/usr/bin/env python3
"""Check macOS-only CI policy; actionlint validates YAML syntax separately."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ACTION_RE = re.compile(r"^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+@[0-9a-f]{40}$")


class PolicyError(ValueError):
    """Raised when workflow text violates the repository's CI policy."""


def check_policy(text: str) -> None:
    runner_lines = re.findall(r"^\s*runs-on:\s*(.*?)\s*$", text, flags=re.MULTILINE)
    if runner_lines != ["macos-15"]:
        raise PolicyError("the workflow must have exactly one macos-15 runner")

    trigger_block = re.search(r"^on:\s*\n(?P<body>(?:\s+.*\n)+)", text, flags=re.MULTILINE)
    if trigger_block is None:
        raise PolicyError("workflow triggers are missing")
    trigger_names = set(re.findall(r"^\s{2}(push|pull_request|workflow_dispatch):", trigger_block["body"], re.MULTILINE))
    if trigger_names != {"push", "pull_request", "workflow_dispatch"}:
        raise PolicyError("workflow must define push, pull_request, and workflow_dispatch")

    if not re.search(r"^permissions:\s*\n\s+contents:\s*read\s*$", text, flags=re.MULTILINE):
        raise PolicyError("workflow must grant contents: read explicitly")
    if re.search(r"\$\{\{\s*secrets\.", text, flags=re.IGNORECASE):
        raise PolicyError("workflow must not reference secrets")
    # The only permitted install is the pinned-purpose reference renderer used by differential tests.
    policy_text = re.sub(r"brew\s+install\s+librsvg\b", "", text)
    if re.search(r"actions/cache|brew\s+install|apt-get|choco\s+install", policy_text, flags=re.IGNORECASE):
        raise PolicyError("workflow must not install packages or configure a cache")

    action_lines = []
    for line in text.splitlines():
        match = re.match(r"^\s*uses:\s*(.*?)\s*$", line)
        if match:
            action_lines.append(match.group(1).split("#", 1)[0].strip())
    if not action_lines or any(not ACTION_RE.fullmatch(item) for item in action_lines):
        raise PolicyError("every external action must be pinned to a full 40-character SHA")


def main(argv: list[str] | None = None) -> int:
    args = sys.argv[1:] if argv is None else argv
    if len(args) != 1:
        print("usage: python3 scripts/check_ci.py WORKFLOW.yml", file=sys.stderr)
        return 2
    try:
        check_policy(Path(args[0]).read_text(encoding="utf-8"))
    except (OSError, PolicyError) as exc:
        print(f"CI policy error: {exc}", file=sys.stderr)
        return 1
    print("macOS-only CI policy: pass")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
