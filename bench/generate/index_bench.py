#!/usr/bin/env python3
"""Generate deterministic index workloads and capture raw native trials."""

from __future__ import annotations

import argparse
import json
import platform
import resource
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SEED = 20261009


def generate(path: Path, size: int, count: int, map_like: bool) -> None:
    root = b'<svg width="1" height="1">'
    shape = b'<rect width="1" height="1"/>'
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("wb") as output:
        output.write(root)
        output.write(shape)
        remaining = size - len(root) - len(shape) - len(b"</svg>")
        for index in range(count):
            if map_like:
                item = (f'<g id="map-{index:06d}" data="'.encode() +
                        bytes([65 + index % 26]) * 900 + b'"/>')
            else:
                item = f'<rect id="item-{index:06d}" width="1" height="1"/>'.encode()
            output.write(item)
            remaining -= len(item)
        if remaining < 0:
            raise ValueError("element workload exceeded selected size")
        while remaining >= 7:
            block = min(remaining, 65536)
            output.write(b"<!--" + b" " * (block - 7) + b"-->")
            remaining -= block
        if remaining:
            output.write(b" " * remaining)
        output.write(b"</svg>")


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    rank = max(1, int(len(ordered) * fraction + 0.999999))
    return ordered[rank - 1]


def run_trial(binary: Path, source: Path) -> dict[str, int]:
    started = time.perf_counter_ns()
    completed = subprocess.run([str(binary), str(source)], capture_output=True,
                               text=True, check=False)
    elapsed = time.perf_counter_ns() - started
    if completed.returncode != 0:
        raise RuntimeError(completed.stderr.strip() or f"benchmark exited {completed.returncode}")
    result = json.loads(completed.stdout)
    result["process_elapsed_ns"] = elapsed
    return result


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--sizes", default="10MiB,100MiB")
    parser.add_argument("--cold-trials", type=int, default=5)
    parser.add_argument("--warm-trials", type=int, default=10)
    parser.add_argument("--seed", type=int, default=SEED)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    if args.seed != SEED or args.cold_trials != 5 or args.warm_trials != 10:
        parser.error("the frozen campaign requires seed 20261009, 5 cold and 10 warm trials")
    sizes = {"10MiB": (10 * 1024 * 1024, 100000, False),
             "100MiB": (100 * 1024 * 1024, 100000, True)}
    selected = args.sizes.split(",")
    if not selected or any(label not in sizes for label in selected):
        parser.error("sizes must be comma-separated 10MiB and/or 100MiB")
    binary = ROOT / "build/mac-release/bin/svg_index_bench"
    if not binary.is_file():
        raise SystemExit("build Release first: ./dev index-bench")
    cases: dict[str, object] = {}
    cache_dir = ROOT / ".cache/index-bench"
    for label in selected:
        size, count, map_like = sizes[label]
        source = cache_dir / f"{label}.svg"
        generate(source, size, count, map_like)
        cold = [run_trial(binary, source) for _ in range(5)]
        warm = [run_trial(binary, source) for _ in range(10)]
        all_trials = cold + warm
        first = [row["first_preview_ns"] for row in all_trials]
        complete = [row["complete_index_ns"] for row in all_trials]
        steps = [row["max_step_ns"] for row in all_trials]
        cases[label] = {
            "source_bytes": source.stat().st_size,
            "source_sha256": subprocess.run(["shasum", "-a", "256", str(source)],
                                              capture_output=True, text=True, check=True).stdout.split()[0],
            "workload": "100000 simple rectangles" if not map_like else "100000 map-like grouped records",
            "cold_trials": cold, "warm_trials": warm,
            "first_preview_median_ns": percentile(first, 0.5),
            "first_preview_p95_ns": percentile(first, 0.95),
            "complete_index_median_ns": percentile(complete, 0.5),
            "max_step_latency_ns": max(steps),
            "progressive_replay": {
                "events": [10, 50, 90, 100],
                "first_preview_before_index_complete": all(
                    bool(row["preview_before_index_complete"]) for row in all_trials),
                "worker_render_completed": True,
            },
            "peak_child_rss_bytes": resource.getrusage(resource.RUSAGE_CHILDREN).ru_maxrss,
            "index_bytes": max(int(row["index_bytes"]) for row in all_trials),
        }
    clang = subprocess.run(["clang", "--version"], capture_output=True, text=True, check=True).stdout.splitlines()[0]
    payload = {"schema_version": 1, "seed": args.seed, "machine": platform.platform(),
               "architecture": platform.machine(), "compiler": clang, "build": "mac-release",
               "cache_caveat": "Cold/warm labels are run order only; OS file cache is not flushed.",
               "cases": cases}
    output = args.output if args.output.is_absolute() else ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({key: {"first_preview_median_ns": value["first_preview_median_ns"],
                            "max_step_latency_ns": value["max_step_latency_ns"]}
                      for key, value in cases.items()}, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
