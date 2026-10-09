#!/usr/bin/env python3
"""Paired Release measurements for the four frozen solid workloads."""
import hashlib
import json
import math
import platform
import runpy
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
workloads = runpy.run_path(str(ROOT / "tests/bench_raster_culling.py"))["workloads"]
CACHE = ROOT / ".cache/M4-06/solid-benchmark-round8"
BASE = Path("/private/tmp/vexel-9bae63e/build/mac-release/bin/shape_render_cli")
CANDIDATE = ROOT / "build/mac-release/bin/shape_render_cli"
OUTPUT = ROOT / "artifacts/WO-M4-06/solid-workloads-round8.json"
TRIALS = 21
WARMUPS = 2


def percentile(values, fraction):
    return sorted(values)[max(0, math.ceil(fraction * len(values)) - 1)]


def mad(values):
    median = statistics.median(values)
    return statistics.median(abs(value - median) for value in values)


def measure(binary, source, output, candidate):
    command = [str(binary)]
    if candidate:
        command.append("--metrics")
    command += [str(source), str(output)]
    started = time.perf_counter()
    run = subprocess.run(command, check=True, capture_output=True, text=True)
    elapsed = time.perf_counter() - started
    peak_rss = None
    if candidate:
        metrics = dict(line.split("=", 1) for line in run.stdout.splitlines()
                       if "=" in line)
        peak_rss = int(metrics["peak_rss_bytes"])
    return elapsed, hashlib.sha256(output.read_bytes()).hexdigest(), peak_rss


def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    all_workloads = workloads()
    rows = {}
    for name, (description, svg) in all_workloads.items():
        source = CACHE / f"{name}.svg"
        source.write_text(svg, encoding="utf-8")
        trial_values = {"base": [], "candidate": []}
        trial_hashes = {"base": [], "candidate": []}
        candidate_rss = []
        for trial in range(WARMUPS + TRIALS):
            order = ("base", "candidate") if trial % 2 == 0 else ("candidate", "base")
            for role in order:
                path = CACHE / f"{name}-{role}-{trial:02d}.rgba"
                elapsed, digest, rss = measure(
                    BASE if role == "base" else CANDIDATE,
                    source, path, role == "candidate")
                if trial >= WARMUPS:
                    trial_values[role].append(elapsed)
                    trial_hashes[role].append(digest)
                    if rss is not None:
                        candidate_rss.append(rss)
        base_times = trial_values["base"]
        candidate_times = trial_values["candidate"]
        base_median = statistics.median(base_times)
        candidate_median = statistics.median(candidate_times)
        overlap = (min(candidate_times) <= max(base_times) and
                   min(base_times) <= max(candidate_times))
        delta = abs(candidate_median - base_median)
        noise_limit = 2.0 * max(mad(base_times), mad(candidate_times))
        rows[name] = {
            "description": description,
            "input_sha256": hashlib.sha256(svg.encode("utf-8")).hexdigest(),
            "output_hashes_identical": len(set(trial_hashes["base"] +
                                                trial_hashes["candidate"])) == 1,
            "output_sha256": trial_hashes["candidate"][0],
            "baseline": {
                "warmups": WARMUPS,
                "trials": TRIALS,
                "trial_seconds": base_times,
                "median_seconds": base_median,
                "p95_seconds": percentile(base_times, 0.95),
                "min_seconds": min(base_times),
                "max_seconds": max(base_times),
                "median_absolute_deviation_seconds": mad(base_times),
                "trial_output_sha256": trial_hashes["base"],
                "peak_rss": "unavailable: frozen-base CLI does not support --metrics",
            },
            "candidate": {
                "warmups": WARMUPS,
                "trials": TRIALS,
                "trial_seconds": candidate_times,
                "median_seconds": candidate_median,
                "p95_seconds": percentile(candidate_times, 0.95),
                "min_seconds": min(candidate_times),
                "max_seconds": max(candidate_times),
                "median_absolute_deviation_seconds": mad(candidate_times),
                "trial_output_sha256": trial_hashes["candidate"],
                "peak_rss_bytes_max": max(candidate_rss),
            },
            "median_change_percent_vs_base":
                (candidate_median / base_median - 1.0) * 100.0,
            "absolute_median_delta_seconds": delta,
            "twice_max_median_absolute_deviation_seconds": noise_limit,
            "observed_ranges_overlap": overlap,
            "within_noise": overlap and delta <= noise_limit,
        }
    result = {
        "work_order": "WO-M4-06",
        "round": 8,
        "baseline_binary": str(BASE),
        "candidate_binary": str(CANDIDATE),
        "host": {"platform": platform.platform(), "machine": platform.machine(),
                 "macos_version": platform.mac_ver()[0] or "unavailable",
                 "compiler": subprocess.run(["clang", "--version"], check=True,
                     capture_output=True, text=True).stdout.splitlines()[0]},
        "pairing": "2 warmups; 21 trials; alternating baseline/candidate order per trial",
        "workloads": rows,
        "all_output_hashes_identical": all(row["output_hashes_identical"]
                                             for row in rows.values()),
        "all_within_noise": all(row["within_noise"] for row in rows.values()),
        "performance_claim": False,
    }
    OUTPUT.write_text(json.dumps(result, indent=2) + "\n", encoding="utf-8")
    print(json.dumps({"workloads": len(rows),
                      "all_output_hashes_identical": result["all_output_hashes_identical"],
                      "all_within_noise": result["all_within_noise"]}))


if __name__ == "__main__":
    main()
