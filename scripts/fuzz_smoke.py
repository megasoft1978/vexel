#!/usr/bin/env python3
"""Deterministic bounded fuzzing of the svgworkbench CLI argument boundary."""

from __future__ import annotations

import argparse
import json
import os
import random
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SANITIZER_RE = re.compile(r"AddressSanitizer|UndefinedBehaviorSanitizer|runtime error:|LeakSanitizer")
PRINTABLE = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 -_./=:+"


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=ROOT / "build/mac-asan/bin/svgworkbench")
    parser.add_argument("--duration", type=float, default=3.0)
    parser.add_argument("--seed", type=int, default=12648430)
    parser.add_argument("--timeout", type=float, default=2.0)
    parser.add_argument("--output", type=Path)
    args = parser.parse_args(argv)
    if not 0 < args.duration <= 60:
        parser.error("--duration must be greater than 0 and at most 60 seconds")
    if not 0 < args.timeout <= 10:
        parser.error("--timeout must be greater than 0 and at most 10 seconds")
    if args.seed < 0:
        parser.error("--seed must be nonnegative")
    return args


def mutation(randomizer: random.Random, iteration: int) -> list[str]:
    base = randomizer.choice(["", "-", "--", "--version", "--help", "file.svg", "💠", "\u00e9"])
    chars = [randomizer.choice(PRINTABLE) for _ in range(randomizer.randint(0, 48))]
    if iteration % 5 == 0:
        chars.insert(randomizer.randrange(len(chars) + 1), randomizer.choice(["é", "雪", "💠"]))
    argument = (base + "".join(chars))[:96]
    return [argument] if iteration % 3 else [argument, "--input=" + argument[:32]]


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    binary = args.binary if args.binary.is_absolute() else ROOT / args.binary
    if not binary.is_file():
        print(f"fuzz binary not found: {binary}", file=sys.stderr)
        return 2
    result_path = args.output or (ROOT / ".cache/fuzz-smoke/latest.json")
    if not result_path.is_absolute():
        result_path = ROOT / result_path
    result_path.parent.mkdir(parents=True, exist_ok=True)
    retain = ROOT / "artifacts/WO-BOOT-05/fuzz-failures"
    randomizer = random.Random(args.seed)
    start = time.monotonic()
    iterations = 0
    failure: str | None = None
    failure_case: dict[str, object] | None = None
    deadline = start + args.duration
    while iterations == 0 or time.monotonic() < deadline:
        case = mutation(randomizer, iterations)
        iterations += 1
        try:
            process = subprocess.run([str(binary), *case], capture_output=True, timeout=args.timeout,
                                     check=False, env={**os.environ, "ASAN_OPTIONS": "detect_leaks=0:abort_on_error=1"})
            stdout = process.stdout.decode("utf-8", errors="replace")
            stderr = process.stderr.decode("utf-8", errors="replace")
            if process.returncode < 0:
                failure = f"signal {abs(process.returncode)}"
            elif SANITIZER_RE.search(stdout + stderr):
                failure = "sanitizer report"
            elif process.returncode == 0 and not (case == ["--version"] and stdout.strip() == "svgworkbench 0.0.1"):
                failure = "unexpected exit 0 for invalid CLI arguments"
            elif process.returncode not in (0, 1):
                failure = f"unexpected exit {process.returncode}"
            if failure:
                failure_case = {"argv": case, "returncode": process.returncode,
                                "stdout": stdout, "stderr": stderr}
        except subprocess.TimeoutExpired as exc:
            failure = "process timeout"
            failure_case = {"argv": case,
                            "stdout": (exc.stdout or b"").decode("utf-8", errors="replace") if isinstance(exc.stdout, bytes) else (exc.stdout or ""),
                            "stderr": (exc.stderr or b"").decode("utf-8", errors="replace") if isinstance(exc.stderr, bytes) else (exc.stderr or "")}
        if failure:
            retain.mkdir(parents=True, exist_ok=True)
            (retain / f"case-{iterations:06d}.json").write_text(json.dumps(failure_case, indent=2) + "\n")
            break
    duration = time.monotonic() - start
    payload = {"seed": args.seed, "iterations": iterations, "duration_seconds": round(duration, 6),
               "requested_duration_seconds": args.duration, "failure": failure}
    result_path.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(payload, sort_keys=True))
    return 1 if failure else 0


if __name__ == "__main__":
    raise SystemExit(main())
