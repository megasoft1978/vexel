#!/usr/bin/env python3
"""Fresh-process raster culling benchmarks against the lane's clean base."""
import argparse
import hashlib
import json
import math
import platform
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BASE_SHA = "bb3d5acd870af36b3bd200e96b4946d809f45a7c"
WARMUPS = 2
TRIALS = 21


def run(argv, cwd=None, capture=False, check=True):
    return subprocess.run(argv, cwd=cwd, check=check, text=True,
                          stdout=subprocess.PIPE if capture else None,
                          stderr=subprocess.PIPE if capture else None)


def percentile95(values):
    ordered = sorted(values)
    return ordered[math.ceil(0.95 * len(ordered)) - 1]


def workloads():
    circle = ('<svg width="640" height="400"><circle cx="170" cy="200" '
              'r="110" fill="red"/></svg>')
    shapes = []
    for index in range(5000):
        x = (index % 100) * 18 + 2
        y = (index // 100) * 20 + 2
        shapes.append(f'<rect x="{x}" y="{y}" width="5" height="5" fill="#336699"/>')
    many = '<svg width="1920" height="1080">' + ''.join(shapes) + '</svg>'
    dashed_lines = []
    for index in range(12):
        y = 45 + index * 82
        dashed_lines.append(
            f'<line x1="20" y1="{y}" x2="1900" y2="{y}" stroke="#a04020" '
            f'stroke-width="2" stroke-linecap="round" stroke-dasharray="128 128" '
            f'stroke-dashoffset="{-index * 3}"/>')
    dashed = '<svg width="1920" height="1080">' + ''.join(dashed_lines) + '</svg>'
    simple_4k = ('<svg width="4096" height="2048"><rect x="128" y="128" '
                 'width="3840" height="1792" fill="#336699"/></svg>')
    return {
        "circle_640x400": ("640x400 single circle r=110", circle),
        "many_shapes_1920x1080": ("1920x1080 with 5,000 5x5 filled rectangles", many),
        "dashed_strokes_1920x1080": ("1920x1080 with 12 long dashed round strokes", dashed),
        "simple_4096x2048": ("4096x2048 simple filled rectangle", simple_4k),
    }


def build_revision(root, with_stats):
    run(["cmake", "--preset", "mac-release"], cwd=root)
    targets = ["shape_render_cli"]
    if with_stats:
        targets.append("shape_render_stats_cli")
    run(["cmake", "--build", "--preset", "mac-release", "--target", *targets], cwd=root)
    cli = root / "build/mac-release/bin/shape_render_cli"
    stats = root / "build/mac-release/bin/shape_render_stats_cli" if with_stats else None
    return cli, stats


def peak_rss(stdout):
    for line in stdout.splitlines():
        if line.startswith("peak_rss_bytes="):
            return int(line.split("=", 1)[1])
    return None


def raster_status(result):
    if result.returncode == 0:
        return "rendered", None
    diagnostic = result.stderr.splitlines()[0] if result.stderr else "process-exit"
    return diagnostic.split(":", 1)[0], diagnostic


def measure(binary, input_path, output_path):
    timings = []
    hashes = []
    rss = []
    for index in range(WARMUPS + TRIALS):
        started = time.perf_counter_ns()
        result = run([str(binary), "--metrics", str(input_path), str(output_path)],
                     capture=True, check=False)
        elapsed = time.perf_counter_ns() - started
        if result.returncode != 0:
            raise RuntimeError(f"measured renderer failed: {result.stderr.strip()}")
        digest = hashlib.sha256(output_path.read_bytes()).hexdigest()
        rss_value = peak_rss(result.stdout)
        if index >= WARMUPS:
            timings.append(elapsed)
            hashes.append(digest)
            if rss_value is not None:
                rss.append(rss_value)
    if len(set(hashes)) != 1:
        raise RuntimeError(f"output hash changed between trials: {hashes}")
    median_ns = int(statistics.median(timings))
    return {
        "status": "rendered",
        "warmups": WARMUPS,
        "trials": TRIALS,
        "trial_latency_ns": timings,
        "median_ms": round(median_ns / 1_000_000.0, 3),
        "p95_ms": round(percentile95(timings) / 1_000_000.0, 3),
        "trial_output_sha256": hashes,
        "output_sha256": hashes[0],
        "peak_rss_bytes": max(rss) if rss else None,
        "peak_rss_status": "getrusage(RUSAGE_SELF) from the child process" if rss else "unavailable",
    }


def baseline_attempt(binary, input_path, output_path):
    probe = run([str(binary), "--metrics", str(input_path), str(output_path)],
                capture=True, check=False)
    status, diagnostic = raster_status(probe)
    if status != "rendered":
        return {
            "status": status,
            "diagnostic": diagnostic,
            "warmups": 0,
            "trials": 0,
            "trial_latency_ns": [],
            "median_ms": None,
            "p95_ms": None,
            "output_sha256": None,
            "trial_output_sha256": [],
            "peak_rss_bytes": peak_rss(probe.stdout),
            "peak_rss_status": "one failed-process measurement; no latency or output comparison",
        }
    return measure(binary, input_path, output_path)


def work_counters(binary, input_path, output_path):
    result = run([str(binary), str(input_path), str(output_path)],
                 capture=True, check=False)
    if result.returncode != 0:
        raise RuntimeError(f"candidate work-counter render failed: {result.stderr.strip()}")
    return {key: int(value) for key, value in
            (line.split("=", 1) for line in result.stderr.splitlines())}


def source_dirty_paths():
    result = run(["git", "-C", str(ROOT), "status", "--porcelain", "--untracked-files=all"],
                 capture=True)
    return [line[3:].split(" -> ")[-1] for line in result.stdout.splitlines()]


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--output", type=Path,
                        default=ROOT / "bench/results/raster-culling.json")
    parser.add_argument("--base-sha", default=BASE_SHA)
    args = parser.parse_args()
    cache = ROOT / ".cache"
    cache.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="raster-culling-bench-", dir=cache) as temp:
        directory = Path(temp)
        base_root = directory / "base-worktree"
        run(["git", "-C", str(ROOT), "worktree", "add", "--detach",
             str(base_root), args.base_sha])
        try:
            base_cli, _ = build_revision(base_root, with_stats=False)
            candidate_cli, candidate_stats = build_revision(ROOT, with_stats=True)
            rows = {}
            for name, (description, source) in workloads().items():
                input_path = directory / f"{name}.svg"
                base_output = directory / f"{name}.base.rgba"
                candidate_output = directory / f"{name}.candidate.rgba"
                stats_output = directory / f"{name}.stats.rgba"
                input_path.write_text(source, encoding="utf-8")
                base = baseline_attempt(base_cli, input_path, base_output)
                candidate = measure(candidate_cli, input_path, candidate_output)
                candidate["work_counters"] = work_counters(candidate_stats,
                                                            input_path, stats_output)
                base_median = base.get("median_ms")
                candidate["median_change_percent_vs_base"] = (
                    round((candidate["median_ms"] - base_median) * 100.0 / base_median, 2)
                    if base_median is not None else None)
                rows[name] = {
                    "description": f"fixed {description}",
                    "input_sha256": hashlib.sha256(source.encode("utf-8")).hexdigest(),
                    "base": base,
                    "candidate": candidate,
                }
            compiler = run(["clang", "--version"], capture=True).stdout.splitlines()[0]
            payload = {
                "schema_version": 1,
                "base_sha": args.base_sha,
                "candidate_git_sha": run(["git", "-C", str(ROOT), "rev-parse", "HEAD"],
                                         capture=True).stdout.strip(),
                "candidate_source_dirty": True,
                "candidate_source_dirty_paths": source_dirty_paths(),
                "host": {"platform": platform.platform(), "machine": platform.machine(),
                         "macos_version": platform.mac_ver()[0], "compiler": compiler},
                "renderer": "Vexel shape_render_cli",
                "measurement": "Release fresh-process SVG decode, raster and raw RGBA write",
                "base_method": "clean detached git worktree at base_sha; renderer status probed before timing",
                "warmups": WARMUPS,
                "measured_trials": TRIALS,
                "rss_method": "shape_render_cli --metrics uses getrusage(RUSAGE_SELF), bytes on macOS",
                "workloads": rows,
            }
        finally:
            run(["git", "-C", str(ROOT), "worktree", "remove", "--force", str(base_root)],
                check=False)
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(json.dumps(payload, indent=2, sort_keys=True) + "\n",
                           encoding="utf-8")
    print(json.dumps({name: {"base": item["base"]["status"],
                             "candidate_median_ms": item["candidate"]["median_ms"],
                             "candidate_p95_ms": item["candidate"]["p95_ms"],
                             "peak_rss_bytes": item["candidate"]["peak_rss_bytes"],
                             "work": item["candidate"]["work_counters"]["total_work"]}
                      for name, item in rows.items()}, sort_keys=True))


if __name__ == "__main__":
    main()
