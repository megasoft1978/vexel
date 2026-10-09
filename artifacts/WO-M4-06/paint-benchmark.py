#!/usr/bin/env python3
"""Fixed-process raster microbenchmarks for WO-M4-06 paint and reference work."""
import hashlib
import json
import math
import platform
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = ROOT / "artifacts/WO-M4-06"
CACHE = ROOT / ".cache/M4-06/paint-bench"
OUTPUT = ROOT / "bench/results/WO-M4-06.json"
CLI = ROOT / "build/mac-release/bin/shape_render_cli"
BASE_CLI = ROOT / ".cache/M4-06/frozen-base/build/mac-release/bin/shape_render_cli"
TRIALS = 21
SIZE = 512


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[max(0, math.ceil(fraction * len(ordered)) - 1)]


def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    sources = {
        "basic-solid": f'<svg width="{SIZE}" height="{SIZE}"><rect width="{SIZE}" height="{SIZE}" fill="#306090"/></svg>',
        "local-use": f'<svg width="{SIZE}" height="{SIZE}"><defs><rect id="r" width="{SIZE}" height="{SIZE}" fill="#306090"/></defs><use href="#r"/></svg>',
        "linear-gradient": f'<svg width="{SIZE}" height="{SIZE}"><defs><linearGradient id="g"><stop stop-color="#ff3020"/><stop offset="1" stop-color="#2030ff"/></linearGradient></defs><rect width="{SIZE}" height="{SIZE}" fill="url(#g)"/></svg>',
        "radial-gradient": f'<svg width="{SIZE}" height="{SIZE}"><defs><radialGradient id="g"><stop stop-color="#303030"/><stop offset="1" stop-color="#404040"/></radialGradient></defs><rect width="{SIZE}" height="{SIZE}" fill="url(#g)"/></svg>',
        "clip-path": f'<svg width="{SIZE}" height="{SIZE}"><defs><clipPath id="c" clipPathUnits="userSpaceOnUse"><rect width="{SIZE // 2}" height="{SIZE}"/></clipPath></defs><rect width="{SIZE}" height="{SIZE}" fill="#306090" clip-path="url(#c)"/></svg>',
    }
    paths = {}
    for name, source in sources.items():
        path = CACHE / f"{name}.svg"
        path.write_text(source)
        paths[name] = path

    result = {
        "work_order": "WO-M4-06",
        "benchmark_status": "complete",
        "work_order_status": "incomplete until corpus, fuzz, safety, and independent review gates pass",
        "renderer": "Vexel scalar 4x4 shape rasterizer",
        "compiler": subprocess.run(["clang", "--version"], check=True, capture_output=True,
                                    text=True).stdout.splitlines()[0],
        "host": {"platform": platform.platform(), "machine": platform.machine(),
                 "macos_version": platform.mac_ver()[0] or "unavailable"},
        "image_dimensions": [SIZE, SIZE],
        "trial_count_per_workload": TRIALS,
        "workloads": [],
        "performance_claim": False,
    }
    for name, source in paths.items():
        times, rss_values, hashes, raw_trials = [], [], [], []
        for trial in range(TRIALS):
            output = CACHE / f"{name}-{trial:02d}.rgba"
            start = time.perf_counter()
            run = subprocess.run([str(CLI), "--metrics", str(source), str(output)],
                                 check=True, capture_output=True, text=True)
            elapsed = time.perf_counter() - start
            metrics = dict(line.split("=", 1) for line in run.stdout.splitlines()
                           if "=" in line)
            raw = output.read_bytes()
            digest = hashlib.sha256(raw).hexdigest()
            rss = int(metrics["peak_rss_bytes"])
            times.append(elapsed)
            rss_values.append(rss)
            hashes.append(digest)
            raw_trials.append({"trial": trial + 1, "seconds": elapsed,
                               "peak_rss_bytes": rss, "output_sha256": digest})
        if len(set(hashes)) != 1:
            raise SystemExit(f"nondeterministic output: {name}")
        median = statistics.median(times)
        p95 = percentile(times, 0.95)
        result["workloads"].append({
            "name": name,
            "trials": raw_trials,
            "median_seconds": median,
            "p95_seconds": p95,
            "throughput_pixels_per_second_median": SIZE * SIZE / median,
            "peak_process_rss_bytes_max": max(rss_values),
            "output_sha256": hashes[0],
            "all_output_hashes_stable": True,
            "frozen_base_comparison": "same workload measured only for basic-solid" if name == "basic-solid" else "not comparable: feature workload was unsupported at frozen base",
        })

    base = {"available": False}
    if BASE_CLI.exists():
        source = paths["basic-solid"]
        samples = []
        base_hashes = []
        for trial in range(TRIALS):
            output = CACHE / f"base-solid-{trial:02d}.rgba"
            start = time.perf_counter()
            subprocess.run([str(BASE_CLI), str(source), str(output)],
                           check=True, capture_output=True, text=True)
            samples.append(time.perf_counter() - start)
            base_hashes.append(hashlib.sha256(output.read_bytes()).hexdigest())
        base = {"available": True, "binary": str(BASE_CLI), "workload": "basic-solid",
                "trials": samples, "median_seconds": statistics.median(samples),
                "p95_seconds": percentile(samples, 0.95),
                "output_sha256": base_hashes[0],
                "stable_output": len(set(base_hashes)) == 1,
                "peak_process_rss": "unavailable: frozen-base CLI does not support --metrics"}
    result["frozen_base_basic_solid"] = base
    result["base_comparison"] = (
        "basic-solid has a separate frozen-base measurement; new reference and paint workloads have no comparable base"
        if base["available"] else "unavailable: frozen-base CLI binary not present"
    )
    OUTPUT.parent.mkdir(parents=True, exist_ok=True)
    OUTPUT.write_text(json.dumps(result, indent=2) + "\n")
    (ARTIFACTS / "paint-benchmark.json").write_text(json.dumps(result, indent=2) + "\n")
    print(json.dumps({"status": result["benchmark_status"], "workloads": len(result["workloads"]),
                      "trials_each": TRIALS, "base_available": base["available"]}))


if __name__ == "__main__":
    main()
