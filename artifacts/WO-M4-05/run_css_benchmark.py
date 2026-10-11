#!/usr/bin/env python3
"""Run fixed CSS-heavy scalar renderer trials and enrich the frozen raster report."""
import hashlib
import json
import platform
import statistics
import subprocess
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "artifacts/WO-M4-05/css-benchmark.json"
CLI = ROOT / "build/mac-release/bin/shape_render_cli"
TRIALS = 21
WIDTH = HEIGHT = 128


def sources():
    boxes = []
    for row in range(8):
        for col in range(8):
            x, y = col * 15 + 4, row * 15 + 4
            cls = "a" if (row + col) % 2 == 0 else "b"
            color = "#336699" if cls == "a" else "#8040a0"
            boxes.append((x, y, cls, color))
    head = f'<svg xmlns="http://www.w3.org/2000/svg" width="{WIDTH}" height="{HEIGHT}">'
    css = (head + '<style>.a { fill: #336699; } .b { fill: #8040a0; } '
           'rect { stroke: none; }</style>' + ''.join(
               f'<rect class="{cls}" x="{x}" y="{y}" width="11" height="11"/>'
               for x, y, cls, _ in boxes) + '</svg>')
    inline = head + ''.join(
        f'<rect style="fill: {color}; stroke: none" x="{x}" y="{y}" width="11" height="11"/>'
        for x, y, _, color in boxes) + '</svg>'
    return {"stylesheet_classes": css, "inline_styles": inline}


def p95(samples):
    return sorted(samples)[(95 * len(samples) + 99) // 100 - 1]


def main():
    legacy = json.loads(OUT.read_text())
    source_paths = ["CMakeLists.txt", "src/svg/css/css.c", "src/svg/css/css.h",
                    "src/svg/shape/shape.c", "src/svg/shape/shape.h",
                    "src/raster/shape_render.c", "tests/svg/css/test_css.c",
                    "tests/svg/shape/shape_render_cli.c",
                    "artifacts/WO-M4-05/run_css_benchmark.py"]
    source_dirty = subprocess.run(["git", "status", "--porcelain", "--", *source_paths],
                                  cwd=ROOT, capture_output=True, text=True, check=True).stdout.strip()
    source_commit = subprocess.run(["git", "log", "-1", "--format=%H", "--",
                                    "src/svg/css/css.c"], cwd=ROOT,
                                   capture_output=True, text=True, check=True).stdout.strip()
    snapshot = hashlib.sha256()
    for relative in source_paths:
        snapshot.update(relative.encode())
        snapshot.update(hashlib.sha256((ROOT / relative).read_bytes()).digest())
    results = {}
    (ROOT / ".cache").mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory(prefix="vexel-css-render-", dir=ROOT / ".cache") as tmp:
        root = Path(tmp)
        for name, source in sources().items():
            svg = root / f"{name}.svg"
            rgba = root / f"{name}.rgba"
            svg.write_text(source)
            timings, hashes, rss_values = [], [], []
            for _ in range(TRIALS):
                started = time.perf_counter_ns()
                run = subprocess.run([str(CLI), "--metrics", str(svg), str(rgba)],
                                     capture_output=True, text=True, check=True)
                timings.append(time.perf_counter_ns() - started)
                data = rgba.read_bytes()
                hashes.append(hashlib.sha256(data).hexdigest())
                rss_values.extend(int(line.split("=", 1)[1]) for line in run.stdout.splitlines()
                                  if line.startswith("peak_rss_bytes="))
            if len(set(hashes)) != 1:
                raise RuntimeError(f"nondeterministic RGBA output: {name}")
            median = int(statistics.median(timings))
            results[name] = {
                "input_sha256": hashlib.sha256(source.encode()).hexdigest(),
                "trial_latency_ns": timings,
                "trials": len(timings),
                "median_latency_ns": median,
                "p95_latency_ns": p95(timings),
                "pixels": WIDTH * HEIGHT,
                "pixels_per_second": round(WIDTH * HEIGHT * 1_000_000_000 / median, 2),
                "output_sha256": hashes[0],
                "trial_output_sha256": hashes,
                "peak_rss_bytes": max(rss_values) if rss_values else None,
                "peak_rss_status": "available via getrusage on macOS" if rss_values else "unavailable",
                "baseline_status": "no baseline: frozen pre-WO renderer has no CSS workload equivalent",
            }
    legacy["css_workloads"] = results
    legacy["css_benchmark_host"] = {"platform": platform.platform(), "machine": platform.machine(),
                                    "macos_version": platform.mac_ver()[0] or "unavailable"}
    legacy["css_benchmark_renderer"] = "Vexel scalar shape_render_cli, Release; decode and raster in fresh process"
    legacy["css_benchmark_trial_count"] = TRIALS
    legacy["css_candidate_base_sha"] = "5fe97cdf3dbe955638d9c5f0d54956ecc1fc2f0c"
    legacy["candidate_sha"] = source_commit
    legacy["candidate_source_dirty"] = bool(source_dirty)
    legacy["candidate_source_dirty_paths"] = source_dirty.splitlines() if source_dirty else []
    legacy["css_candidate_source_status"] = (
        "dirty source tree" if source_dirty else "clean committed source tree")
    legacy["css_candidate_commit_sha"] = source_commit
    legacy["css_candidate_source_snapshot_sha256"] = snapshot.hexdigest()
    OUT.write_text(json.dumps(legacy, indent=2, sort_keys=True) + "\n")
    print(json.dumps({name: {"median_ns": value["median_latency_ns"],
                             "p95_ns": value["p95_latency_ns"],
                             "sha256": value["output_sha256"]}
                      for name, value in results.items()}, sort_keys=True))


if __name__ == "__main__":
    main()
