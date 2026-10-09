#!/usr/bin/env python3
"""Run the bounded XML tokenizer ASan generator and retain its JSON result."""
import argparse
import json
from pathlib import Path
import subprocess
import time

ROOT = Path(__file__).resolve().parents[1]


def confined_output_path(root, value):
    output = (root / value).resolve()
    allowed_roots = ((root / ".cache").resolve(),
                     (root / "artifacts/WO-M2-02").resolve())
    if not any(output.is_relative_to(allowed) for allowed in allowed_roots):
        raise ValueError("--output must stay under .cache/ or artifacts/WO-M2-02/")
    return output


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True)
    parser.add_argument("--duration", required=True, type=int)
    parser.add_argument("--seed", required=True, type=int)
    parser.add_argument("--output", required=True)
    args = parser.parse_args()
    binary = (ROOT / args.binary).resolve()
    if not binary.is_relative_to(ROOT):
        parser.error("--binary must stay inside the repository")
    if not binary.is_file() and binary == (ROOT / "build/mac-asan/bin/xml_tokenizer_fuzz"):
        subprocess.run(["./dev", "asan"], cwd=ROOT, check=True)
    if not binary.is_file():
        parser.error(f"fuzzer binary not found: {binary}")
    started = time.monotonic()
    run = subprocess.run([str(binary), str(args.seed), str(args.duration)], cwd=ROOT,
                         check=True, capture_output=True, text=True, timeout=args.duration + 30)
    elapsed = time.monotonic() - started
    report = json.loads(run.stdout)
    if report["seed"] != args.seed or report["elapsed_seconds"] + 0.005 < args.duration:
        raise RuntimeError("fuzzer did not complete the full requested duration")
    if report["sanitizer_status"] != "clean" or report["iterations"] <= 0:
        raise RuntimeError("fuzzer invariant or sanitizer status failed")
    report["wall_elapsed_seconds"] = round(elapsed, 6)
    report["duration_requested_seconds"] = args.duration
    try:
        out = confined_output_path(ROOT, args.output)
    except ValueError as error:
        parser.error(str(error))
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
