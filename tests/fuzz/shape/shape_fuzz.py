#!/usr/bin/env python3
"""Run a fixed-seed, bounded shape parser/render fuzz campaign."""
import argparse
import json
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--duration", type=int, default=60)
    parser.add_argument("--seed", type=int, required=True)
    parser.add_argument("--cases", type=int, default=1_000_000)
    parser.add_argument("--output", type=Path, default=ROOT / "artifacts/WO-M2-05/fuzz-report.json")
    args = parser.parse_args()
    if not 1 <= args.duration <= 600 or not 1 <= args.cases <= 100_000_000 or not 0 <= args.seed <= 0xFFFFFFFF:
        parser.error("duration, case count, or seed is outside the configured bounds")
    binary = args.binary.resolve()
    if not binary.is_file():
        parser.error(f"fuzz binary does not exist: {binary}")
    start = time.monotonic()
    run = subprocess.run([str(binary), str(args.seed), str(args.cases), str(args.duration)], cwd=ROOT,
                         capture_output=True, text=True, timeout=args.duration + 10, check=False)
    elapsed = time.monotonic() - start
    stderr_path = args.output.with_suffix(".stderr.txt")
    stderr_path.parent.mkdir(parents=True, exist_ok=True)
    stderr_path.write_text(run.stderr)
    if run.returncode != 0:
        raise RuntimeError(f"fuzz process exited {run.returncode}; stderr retained at {stderr_path}")
    if run.stderr:
        raise RuntimeError(f"fuzz process emitted sanitizer stderr; retained at {stderr_path}")
    fields = dict(item.split("=", 1) for item in run.stdout.strip().split())
    cases = int(fields["cases"])
    campaign_seconds = float(fields["duration"])
    cap_exhaustion = int(fields.get("cap_exhaustion", "0"))
    cap_case_bytes = int(fields.get("cap_case_bytes", "0"))
    if cases < 1_000_000 and campaign_seconds < args.duration:
        raise RuntimeError("campaign did not reach either required floor")
    if cap_exhaustion != 1 or cap_case_bytes < 1 or cap_case_bytes > 16 * 1024 * 1024:
        raise RuntimeError("bounded command-cap exhaustion case did not pass")
    report = {"seed": args.seed, "cases": cases, "duration_seconds": round(elapsed, 3),
              "process_cpu_seconds": campaign_seconds,
              "binary": str(binary), "sanitizer": "AddressSanitizer + UndefinedBehaviorSanitizer",
              "sanitizer_stderr": str(stderr_path), "sanitizer_stderr_bytes": len(run.stderr.encode()),
              "random_input_max_bytes": 4096, "cap_exhaustion_cases": cap_exhaustion,
              "cap_exhaustion_input_bytes": cap_case_bytes,
              "input_max_bytes": max(4096, cap_case_bytes),
              "crashes": 0, "sanitizer_reports": 0,
              "invariant_failures": 0, "result": "pass"}
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
