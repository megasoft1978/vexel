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


def median_absolute_deviation(values):
    median = statistics.median(values)
    return statistics.median(abs(value - median) for value in values)


def main():
    CACHE.mkdir(parents=True, exist_ok=True)
    gradient_shapes = "".join(
        f'<rect x="{(index % 25) * 20 + 1}" y="{(index // 25) * 25 + 1}" '
        'width="18" height="18" fill="url(#g)"/>'
        for index in range(500))
    sources = {
        "basic-solid": f'<svg width="{SIZE}" height="{SIZE}"><rect width="{SIZE}" height="{SIZE}" fill="#306090"/></svg>',
        "local-use": f'<svg width="{SIZE}" height="{SIZE}"><defs><rect id="r" width="{SIZE}" height="{SIZE}" fill="#306090"/></defs><use href="#r"/></svg>',
        "linear-gradient": f'<svg width="{SIZE}" height="{SIZE}"><defs><linearGradient id="g"><stop stop-color="#ff3020"/><stop offset="1" stop-color="#2030ff"/></linearGradient></defs><rect width="{SIZE}" height="{SIZE}" fill="url(#g)"/></svg>',
        "radial-gradient": f'<svg width="{SIZE}" height="{SIZE}"><defs><radialGradient id="g"><stop stop-color="#303030"/><stop offset="1" stop-color="#404040"/></radialGradient></defs><rect width="{SIZE}" height="{SIZE}" fill="url(#g)"/></svg>',
        "clip-path": f'<svg width="{SIZE}" height="{SIZE}"><defs><clipPath id="c" clipPathUnits="userSpaceOnUse"><rect width="{SIZE // 2}" height="{SIZE}"/></clipPath></defs><rect width="{SIZE}" height="{SIZE}" fill="#306090" clip-path="url(#c)"/></svg>',
        "gradient-heavy-500-shapes": (
            f'<svg width="{SIZE}" height="{SIZE}"><defs>'
            '<linearGradient id="g" x1="0" y1="0" x2="0.22" y2="1">'
            '<stop stop-color="#ff3020"/><stop offset="0.45" stop-color="#20d060"/>'
            '<stop offset="1" stop-color="#2030ff"/></linearGradient></defs>'
            f'{gradient_shapes}</svg>'),
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
            "min_seconds": min(times),
            "max_seconds": max(times),
            "median_absolute_deviation_seconds": median_absolute_deviation(times),
            "throughput_pixels_per_second_median": SIZE * SIZE / median,
            "peak_process_rss_bytes_max": max(rss_values),
            "output_sha256": hashes[0],
            "all_output_hashes_stable": True,
            "frozen_base_comparison": "same workload measured only for basic-solid" if name == "basic-solid" else "not comparable: feature workload was unsupported at frozen base",
        })

    base = {"available": False}
    if BASE_CLI.exists():
        source = paths["basic-solid"]
        samples = {"base": [], "candidate": []}
        hashes = {"base": [], "candidate": []}
        rss_values = []
        for trial in range(TRIALS + 2):
            order = ("base", "candidate") if trial % 2 == 0 else ("candidate", "base")
            for role in order:
                output = CACHE / f"paired-{role}-solid-{trial:02d}.rgba"
                if role == "candidate":
                    command = [str(CLI), "--metrics", str(source), str(output)]
                else:
                    command = [str(BASE_CLI), str(source), str(output)]
                start = time.perf_counter()
                run = subprocess.run(command, check=True, capture_output=True, text=True)
                elapsed = time.perf_counter() - start
                digest = hashlib.sha256(output.read_bytes()).hexdigest()
                if trial >= 2:
                    samples[role].append(elapsed)
                    hashes[role].append(digest)
                    if role == "candidate":
                        metrics = dict(line.split("=", 1) for line in run.stdout.splitlines()
                                       if "=" in line)
                        rss_values.append(int(metrics["peak_rss_bytes"]))
        current = result["workloads"][0]
        current_times = samples["candidate"]
        current["trials"] = [
            {"trial": index + 1, "seconds": elapsed,
             "peak_rss_bytes": rss_values[index],
             "output_sha256": hashes["candidate"][index]}
            for index, elapsed in enumerate(current_times)]
        current["median_seconds"] = statistics.median(current_times)
        current["p95_seconds"] = percentile(current_times, 0.95)
        current["min_seconds"] = min(current_times)
        current["max_seconds"] = max(current_times)
        current["median_absolute_deviation_seconds"] = median_absolute_deviation(current_times)
        current["throughput_pixels_per_second_median"] = SIZE * SIZE / current["median_seconds"]
        current["peak_process_rss_bytes_max"] = max(rss_values)
        current["output_sha256"] = hashes["candidate"][0]
        current["all_output_hashes_stable"] = len(set(hashes["candidate"])) == 1
        base = {"available": True, "binary": str(BASE_CLI), "workload": "basic-solid",
                "warmups": 2, "trials": samples["base"],
                "median_seconds": statistics.median(samples["base"]),
                "p95_seconds": percentile(samples["base"], 0.95),
                "min_seconds": min(samples["base"]), "max_seconds": max(samples["base"]),
                "median_absolute_deviation_seconds": median_absolute_deviation(samples["base"]),
                "output_sha256": hashes["base"][0],
                "stable_output": len(set(hashes["base"])) == 1,
                "peak_process_rss": "unavailable: frozen-base CLI does not support --metrics"}
    result["frozen_base_basic_solid"] = base
    if base["available"]:
        current_solid = result["workloads"][0]
        current_median = current_solid["median_seconds"]
        base_median = base["median_seconds"]
        ranges_overlap = (current_solid["min_seconds"] <= base["max_seconds"] and
                          base["min_seconds"] <= current_solid["max_seconds"])
        median_delta = abs(current_median - base_median)
        noise_limit = 2.0 * max(
            current_solid["median_absolute_deviation_seconds"],
            base["median_absolute_deviation_seconds"])
        result["basic_solid_noise_comparison"] = {
            "candidate_median_change_percent":
                (current_median / base_median - 1.0) * 100.0,
            "absolute_median_delta_seconds": median_delta,
            "twice_max_median_absolute_deviation_seconds": noise_limit,
            "observed_ranges_overlap": ranges_overlap,
            "within_noise": ranges_overlap and median_delta <= noise_limit,
            "output_hashes_identical":
                current_solid["output_sha256"] == base["output_sha256"],
        }
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
