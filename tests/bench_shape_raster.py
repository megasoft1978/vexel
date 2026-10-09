#!/usr/bin/env python3
"""Repeatable CLI raster benchmark against the frozen base revision."""
import argparse
import hashlib
import json
import platform
import shutil
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE_SHA = "6cd61dd6570ab556af7d6a1feb8899d3f849f8af"
WARMUPS = 2
TRIALS = 21


def run(argv, cwd=None, capture=False):
    return subprocess.run(argv, cwd=cwd, check=True, text=True,
                          stdout=subprocess.PIPE if capture else None,
                          stderr=subprocess.PIPE if capture else None)


def workload_sources():
    width = height = 128
    simple = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
              f'viewBox="0 0 {width} {height}"><rect x="8" y="8" width="112" height="112" '
              'fill="#336699"/></svg>')
    curved = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
              f'viewBox="0 0 {width} {height}"><path fill="#8040a0" '
              'd="M8 64 C8 8 120 8 120 64 C120 120 8 120 8 64 Z"/></svg>')
    shapes = []
    for row in range(4):
        for column in range(4):
            x, y = 4 + column * 30, 4 + row * 30
            color = ((row * 67 + column * 13) & 255,
                     (column * 53 + row * 19) & 255,
                     (row * 31 + column * 71) & 255)
            shapes.append(f'<rect x="{x}.25" y="{y}.5" width="24.5" height="24.25" '
                          f'fill="#{color[0]:02x}{color[1]:02x}{color[2]:02x}"/>')
    multi = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
             f'viewBox="0 0 {width} {height}">' + "".join(shapes) + "</svg>")
    return {"simple": simple, "curved": curved, "multi_shape": multi}


def build_revision(root):
    run(["cmake", "--preset", "mac-release"], cwd=root)
    run(["cmake", "--build", "--preset", "mac-release", "--target", "shape_render_cli"], cwd=root)
    return root / "build/mac-release/bin/shape_render_cli"


def percentile95(values):
    ordered = sorted(values)
    return ordered[max(0, (95 * len(ordered) + 99) // 100 - 1)]


def validate_payload(payload):
    expected = {"simple", "curved", "multi_shape"}
    if payload.get("schema_version") != 1 or set(payload.get("workloads", {})) != expected:
        raise ValueError("benchmark workload schema mismatch")
    for name, workload in payload["workloads"].items():
        if len(workload["input_sha256"]) != 64:
            raise ValueError(f"invalid input hash: {name}")
        if not workload["description"].startswith("fixed "):
            raise ValueError(f"missing deterministic workload description: {name}")
        for revision in ("base", "candidate"):
            result = workload[revision]
            if (result["warmups"] != WARMUPS or result["trials"] != TRIALS or
                    len(result["trial_latency_ns"]) != TRIALS or
                    len(result["output_sha256"]) != 64 or
                    len(set(result["trial_output_sha256"])) != 1):
                raise ValueError(f"invalid measured trial data: {name}/{revision}")


def measure(binary, source, temp, label):
    input_path = temp / f"{label}.svg"
    output_path = temp / f"{label}.rgba"
    input_path.write_text(source, encoding="utf-8")
    timings, hashes, rss = [], [], []
    for index in range(WARMUPS + TRIALS):
        start = time.perf_counter_ns()
        result = run([str(binary), "--metrics", str(input_path), str(output_path)], capture=True)
        elapsed = time.perf_counter_ns() - start
        if index >= WARMUPS:
            raw = output_path.read_bytes()
            timings.append(elapsed)
            hashes.append(hashlib.sha256(raw).hexdigest())
            for line in result.stdout.splitlines():
                if line.startswith("peak_rss_bytes="):
                    rss.append(int(line.split("=", 1)[1]))
    if len(set(hashes)) != 1:
        raise RuntimeError(f"nondeterministic output from {binary}: {hashes}")
    median_ns = int(statistics.median(timings))
    pixels = 128 * 128
    return {"warmups": WARMUPS, "trials": TRIALS, "trial_latency_ns": timings,
            "median_latency_ns": median_ns, "p95_latency_ns": percentile95(timings),
            "pixels": pixels, "pixels_per_second": round(pixels * 1_000_000_000 / median_ns, 2),
            "output_sha256": hashes[0], "trial_output_sha256": hashes,
            "peak_rss_bytes": max(rss) if rss else None,
            "peak_rss_status": "available via getrusage on macOS" if rss else "unavailable"}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    cmake = shutil.which("cmake")
    if not cmake:
        raise SystemExit("cmake is required")
    compiler = run(["clang", "--version"], capture=True).stdout.splitlines()[0]
    workloads = workload_sources()
    with tempfile.TemporaryDirectory(prefix="vexel-m4-03-bench-") as directory:
        temp = Path(directory)
        clone = temp / "benchmark-repository"
        base_root = temp / "base-worktree"
        run(["git", "clone", "--shared", "--no-checkout", str(ROOT), str(clone)])
        run(["git", "-C", str(clone), "worktree", "add", "--detach", str(base_root), BASE_SHA])
        base_binary = build_revision(base_root)
        candidate_binary = build_revision(ROOT)
        results = {}
        for name, source in workloads.items():
            results[name] = {"description": f"fixed {name} SVG, 128x128 viewport; deterministic geometry and colors",
                             "input_sha256": hashlib.sha256(source.encode()).hexdigest(),
                             "base": measure(base_binary, source, temp, f"{name}-base"),
                             "candidate": measure(candidate_binary, source, temp, f"{name}-candidate")}
            old = results[name]["base"]["median_latency_ns"]
            new = results[name]["candidate"]["median_latency_ns"]
            results[name]["median_change_percent"] = round((new - old) * 100.0 / old, 2)
        payload = {"schema_version": 1, "base_sha": BASE_SHA,
                   "candidate_sha": run(["git", "-C", str(ROOT), "rev-parse", "HEAD"], capture=True).stdout.strip(),
                   "candidate_worktree_dirty": bool(run(["git", "-C", str(ROOT), "status", "--porcelain"], capture=True).stdout.strip()),
                   "host": {"platform": platform.platform(), "machine": platform.machine(),
                            "macos_version": platform.mac_ver()[0],
                            "hardware_model": "unavailable: model query is not part of the benchmark harness",
                            "compiler": compiler},
                   "renderer": "Vexel shape_render_cli", "measurement": "end-to-end process, SVG decode, and raster; Release build",
                   "baseline_method": "isolated temporary clone with detached git worktree at base_sha",
                   "warmups": WARMUPS, "measured_trials": TRIALS,
                   "rss_method": "shape_render_cli getrusage(RUSAGE_SELF), ru_maxrss bytes on macOS",
                   "workloads": results}
        validate_payload(payload)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({name: item["median_change_percent"] for name, item in results.items()}, sort_keys=True))


if __name__ == "__main__":
    main()
