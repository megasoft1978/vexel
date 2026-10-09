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
BASE_SHA = "1e9c7b81f1bda074031af4fd165240e8d5eeba09"
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
    line = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
            f'viewBox="0 0 {width} {height}"><line x1="8" y1="64" x2="120" y2="64" '
            'stroke="#336699" stroke-width="3" stroke-linecap="round"/></svg>')
    curve = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
             f'viewBox="0 0 {width} {height}"><circle cx="64" cy="64" r="48" fill="none" '
             'stroke="#8040a0" stroke-width="2" stroke-linejoin="round"/></svg>')
    dashed = (f'<svg xmlns="http://www.w3.org/2000/svg" width="{width}" height="{height}" '
              f'viewBox="0 0 {width} {height}"><polyline points="8 112 32 16 64 112 96 16 120 112" '
              'fill="none" stroke="#a04020" stroke-width="2" stroke-linejoin="round" '
              'stroke-dasharray="7 3 2" stroke-dashoffset="-1"/></svg>')
    return {"simple": simple, "curved": curved, "multi_shape": multi,
            "line_stroke": line, "curve_stroke": curve, "dashed_stroke": dashed}


def build_revision(root):
    run(["cmake", "--preset", "mac-release"], cwd=root)
    run(["cmake", "--build", "--preset", "mac-release", "--target", "shape_render_cli"], cwd=root)
    return root / "build/mac-release/bin/shape_render_cli"


def source_dirty_paths(root):
    result = run(["git", "-C", str(root), "status", "--porcelain", "--untracked-files=all"], capture=True)
    paths = [line[3:].split(" -> ")[-1] for line in result.stdout.splitlines()]
    roots = ("src/", "cmake/", "tests/bench_shape_raster.py")
    exact = {"CMakeLists.txt", "CMakePresets.json", "tests/svg/shape/shape_render_cli.c"}
    return sorted(path for path in paths if path in exact or path.startswith(roots))


def percentile95(values):
    ordered = sorted(values)
    return ordered[max(0, (95 * len(ordered) + 99) // 100 - 1)]


def validate_payload(payload):
    expected = {"simple", "curved", "multi_shape", "line_stroke", "curve_stroke", "dashed_stroke"}
    if payload.get("schema_version") != 1 or set(payload.get("workloads", {})) != expected:
        raise ValueError("benchmark workload schema mismatch")
    for name, workload in payload["workloads"].items():
        if len(workload["input_sha256"]) != 64:
            raise ValueError(f"invalid input hash: {name}")
        if not workload["description"].startswith("fixed "):
            raise ValueError(f"missing deterministic workload description: {name}")
        for revision in ("base", "candidate"):
            result = workload[revision]
            if revision == "base" and str(result.get("status", "")).startswith("unsupported"):
                continue
            if (result["warmups"] != WARMUPS or result["trials"] != TRIALS or
                    len(result["trial_latency_ns"]) != TRIALS or
                    len(result["output_sha256"]) != 64 or
                    len(set(result["trial_output_sha256"])) != 1):
                raise ValueError(f"invalid measured trial data: {name}/{revision}")


def measure_pair(base_binary, candidate_binary, source, temp, label):
    input_path = temp / f"{label}.svg"
    output_path = temp / f"{label}.rgba"
    input_path.write_text(source, encoding="utf-8")
    base_probe = subprocess.run([str(base_binary), "--metrics", str(input_path), str(output_path)],
                                capture_output=True, text=True, check=False)
    base_available = base_probe.returncode == 0
    samples = {"base": {"timings": [], "hashes": [], "rss": []},
               "candidate": {"timings": [], "hashes": [], "rss": []}}
    binaries = {"base": base_binary, "candidate": candidate_binary}
    for index in range(WARMUPS + TRIALS):
        order = ("base", "candidate") if index % 2 == 0 else ("candidate", "base")
        for revision in order:
            if revision == "base" and not base_available:
                continue
            binary = binaries[revision]
            started = time.perf_counter_ns()
            result = run([str(binary), "--metrics", str(input_path), str(output_path)], capture=True)
            elapsed = time.perf_counter_ns() - started
            if index >= WARMUPS:
                sample = samples[revision]
                sample["timings"].append(elapsed)
                sample["hashes"].append(hashlib.sha256(output_path.read_bytes()).hexdigest())
                for line in result.stdout.splitlines():
                    if line.startswith("peak_rss_bytes="):
                        sample["rss"].append(int(line.split("=", 1)[1]))
    results = {}
    for revision, sample in samples.items():
        if revision == "base" and not base_available:
            results[revision] = {"status": "unsupported: frozen base rejects stroke attributes",
                                 "warmups": 0, "trials": 0, "trial_latency_ns": [],
                                 "median_latency_ns": None, "p95_latency_ns": None,
                                 "pixels": 128*128, "pixels_per_second": None,
                                 "output_sha256": None, "trial_output_sha256": [],
                                 "peak_rss_bytes": None,
                                 "peak_rss_status": "unavailable: frozen base rejects workload"}
            continue
        if len(set(sample["hashes"])) != 1:
            raise RuntimeError(f"nondeterministic output from {binaries[revision]}: {sample['hashes']}")
        timings, hashes, rss = sample["timings"], sample["hashes"], sample["rss"]
        median_ns = int(statistics.median(timings))
        results[revision] = {
            "warmups": WARMUPS, "trials": TRIALS, "trial_latency_ns": timings,
            "median_latency_ns": median_ns, "p95_latency_ns": percentile95(timings),
            "pixels": 128 * 128,
            "pixels_per_second": round(128 * 128 * 1_000_000_000 / median_ns, 2),
            "output_sha256": hashes[0], "trial_output_sha256": hashes,
            "peak_rss_bytes": max(rss) if rss else None,
            "peak_rss_status": "available via getrusage on macOS" if rss else "unavailable"}
    return results


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
            pair = measure_pair(base_binary, candidate_binary, source, temp, name)
            results[name] = {"description": f"fixed {name} SVG, 128x128 viewport; deterministic geometry and colors",
                             "input_sha256": hashlib.sha256(source.encode()).hexdigest(),
                             **pair}
            old = results[name]["base"]["median_latency_ns"]
            new = results[name]["candidate"]["median_latency_ns"]
            results[name]["median_change_percent"] = (round((new - old) * 100.0 / old, 2)
                                                       if old is not None else None)
        dirty_paths = source_dirty_paths(ROOT)
        # Evidence-only commits must not make the measured raster revision appear newer.
        source_revision = run(["git", "-C", str(ROOT), "log", "-1", "--format=%H", "--",
                               "src/raster/shape_render.c", "tests/svg/shape/shape_render_cli.c",
                               "CMakeLists.txt"], capture=True).stdout.strip()
        payload = {"schema_version": 1, "base_sha": BASE_SHA,
                   "candidate_sha": source_revision,
                   "candidate_source_dirty": bool(dirty_paths),
                   "candidate_source_dirty_paths": dirty_paths,
                   "host": {"platform": platform.platform(), "machine": platform.machine(),
                            "macos_version": platform.mac_ver()[0],
                            "hardware_model": "unavailable: model query is not part of the benchmark harness",
                            "compiler": compiler},
                   "renderer": "Vexel shape_render_cli", "measurement": "end-to-end process, SVG decode, and raster; Release build",
                   "baseline_method": "isolated temporary clone with detached git worktree at base_sha; paired trials alternate base/candidate order",
                   "warmups": WARMUPS, "measured_trials": TRIALS,
                   "rss_method": "shape_render_cli getrusage(RUSAGE_SELF), ru_maxrss bytes on macOS",
                   "workloads": results}
        validate_payload(payload)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(json.dumps({name: item["median_change_percent"] for name, item in results.items()}, sort_keys=True))


if __name__ == "__main__":
    main()
