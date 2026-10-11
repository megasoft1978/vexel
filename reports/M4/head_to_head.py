#!/usr/bin/env python3
"""Run and validate M4 corpus coverage using the established progressive index harness."""

from __future__ import annotations

import argparse
import importlib.util
import json
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
M3_SCRIPT = ROOT / "reports/M3/head_to_head.py"
RAW = ROOT / ".cache/M4/head-to-head-raw.json"
POINTER = ROOT / ".cache/M4/corpus-pointer.json"
REPORT = ROOT / "reports/M4/head-to-head.json"
MARKDOWN = ROOT / "reports/M4/head-to-head.md"
PERF = ROOT / "docs/PERF.md"
PERF_START = "<!-- M4-01 generated report start -->"
PERF_END = "<!-- M4-01 generated report end -->"


def load_harness():
    spec = importlib.util.spec_from_file_location("m3_head_to_head", M3_SCRIPT)
    if spec is None or spec.loader is None:
        raise RuntimeError("cannot load the frozen M3 progressive index harness")
    module = importlib.util.module_from_spec(spec)
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


def render(harness, raw: dict) -> tuple[str, str]:
    report = harness.aggregate(raw)
    report["environment"]["binary_path"] = "build/mac-release/bin/svg_index_bench"
    size_paths = {
        "debug_svgworkbench": ROOT / "build/mac-debug/bin/svgworkbench",
        "release_svgworkbench": ROOT / "build/mac-release/bin/svgworkbench",
        "release_svg_index_bench": ROOT / "build/mac-release/bin/svg_index_bench",
        "debug_shape_render_cli": ROOT / "build/mac-debug/bin/shape_render_cli",
        "release_shape_render_cli": ROOT / "build/mac-release/bin/shape_render_cli",
        "debug_libvexel_shape": ROOT / "build/mac-debug/libvexel_shape.a",
        "release_libvexel_shape": ROOT / "build/mac-release/libvexel_shape.a",
    }
    report["binary_sizes_bytes"] = {
        name: path.stat().st_size for name, path in size_paths.items()
    }
    markdown = harness.render_markdown(report)
    report["results"] = [
        {
            **{key: row[key] for key in ("id", "path", "tier", "bytes", "sha256", "outcome", "trial_outcomes")},
            "metrics": row["metrics"],
            "inkscape": row["inkscape"],
        }
        for row in report["results"]
    ]
    return json.dumps(report, indent=2) + "\n", markdown


def perf_block(report: dict) -> str:
    env = report["environment"]
    sizes = report["binary_sizes_bytes"]
    tiers = report["tier_summary"]
    gates = report["wedge_gates"]
    t1 = tiers["T1"]
    successes = [row for row in report["results"] if row["tier"] == "T1" and row["outcome"] == "ok"]
    successful_metrics = [
        f"{row['id']}: first preview {row['metrics']['first_preview_ns'] / 1e6:.3f} ms, "
        f"full index {row['metrics']['complete_index_ns'] / 1e6:.3f} ms"
        for row in successes
    ]
    preview = gates["first_visible_preview_100_mib"]
    step = gates["maximum_main_thread_step"]
    rss = gates["peak_rss_100_mib"]
    time_ratio = t1["inkscape_seconds_over_vexel_first_preview_geomean"]
    rss_ratio = t1["inkscape_rss_over_vexel_rss_geomean"]
    ratio_text = (f"The {t1['comparable_successful_files']} comparable successful T1 rows have an Inkscape/Vexel "
                  f"first-preview geometric mean of {time_ratio:.3f}x and RSS ratio of {rss_ratio:.3f}x."
                  if time_ratio is not None and rss_ratio is not None else
                  f"T1 comparison ratios are unavailable ({t1['comparable_successful_files']} comparable successful files).")
    return "\n".join([
        "## M4 cumulative shape and corpus workload",
        "",
        f"Measured on {env['architecture']} {env['macos']}, {env['chip']}, {env['compiler']}.",
        f"Debug/Release `svgworkbench`: {sizes['debug_svgworkbench']} / {sizes['release_svgworkbench']} bytes; "
        f"Release `svg_index_bench`: {sizes['release_svg_index_bench']} bytes;",
        f"Debug/Release `shape_render_cli`: {sizes['debug_shape_render_cli']} / {sizes['release_shape_render_cli']} bytes; "
        f"Debug/Release `libvexel_shape.a`: {sizes['debug_libvexel_shape']} / {sizes['release_libvexel_shape']} bytes.",
        "Binary sizes are also present in the JSON report and verified by `./dev bench-corpus --check`.",
        "",
        f"The 58-file run produced {sum(t['ok'] for t in tiers.values())} previewable, "
        f"{sum(t['unsupported'] for t in tiers.values())} unsupported, "
        f"{sum(t['limit'] for t in tiers.values())} limit, and {sum(t['crash'] for t in tiers.values())} crash outcomes.",
        ratio_text + " T2–T4 ratios are unavailable when Vexel has no successful outcomes there.",
        "These are directional open/index comparisons against the supplied Inkscape CLI baseline, not full-render parity.",
        f"The 100 MiB first-preview gate is {preview['status']} ({preview['measured_files']}/{preview['eligible_t3_files']} eligible files); "
        f"maximum main-thread step is {step['max_step_ns'] / 1e6:.3f} ms "
        f"({step['status']}); 100 MiB RSS gate is {rss['status']} "
        f"({sum(1 for row in rss['files'] if row['within_limit'])}/{len(rss['files'])} within the per-file allowance).",
        "Successful T1 measurements: " + ("; ".join(successful_metrics) if successful_metrics else "none"),
        "Per-file results and aggregate timings are in `reports/M4/head-to-head.json` and `.md`; raw trials and the external manifest pointer stay in ignored `.cache/M4/`.",
    ])


def update_perf(report: dict) -> str:
    text = PERF.read_text(encoding="utf-8")
    if text.count(PERF_START) != 1 or text.count(PERF_END) != 1:
        raise ValueError("docs/PERF.md must contain one M4 generated-section marker pair")
    start = text.index(PERF_START) + len(PERF_START)
    end = text.index(PERF_END)
    if end < start:
        raise ValueError("docs/PERF.md M4 generated-section markers are reversed")
    return text[:start] + "\n\n" + perf_block(report) + "\n" + text[end:]


def run(harness) -> None:
    original_classify = harness.classify
    time_wrapper_fallback = {"used": False}

    def classify_with_time_fallback(result, timed_out):
        if "time: sysctl kern.clockrate: Operation not permitted" in result.stderr:
            try:
                measured = json.loads(result.stdout)
            except (json.JSONDecodeError, TypeError):
                measured = {}
            if measured.get("outcome") in ("ok", "unsupported", "limit"):
                time_wrapper_fallback["used"] = True
                return measured["outcome"]
        return original_classify(result, timed_out)

    harness.classify = classify_with_time_fallback
    raw = harness.run_campaign()
    if time_wrapper_fallback["used"]:
        raw["environment"]["process_policy"] = (
            "fresh Vexel process per trial; /usr/bin/time -l emitted child metrics but could not query "
            "kern.clockrate in this sandbox; peak RSS uses the benchmark process getrusage value")
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.write_text(json.dumps(raw, indent=2) + "\n", encoding="utf-8")
    shutil.copyfile(harness.POINTER, POINTER)
    report_json, report_md = render(harness, raw)
    report = json.loads(report_json)
    REPORT.write_text(report_json, encoding="utf-8")
    MARKDOWN.write_text(report_md, encoding="utf-8")
    PERF.write_text(update_perf(report), encoding="utf-8")
    print(f"reported {len(raw['results'])} files / "
          f"{sum(len(row['trials']) for row in raw['results'])} fresh trials")


def check(harness) -> None:
    raw = json.loads(RAW.read_text(encoding="utf-8"))
    if json.loads(POINTER.read_text(encoding="utf-8")) != json.loads(harness.POINTER.read_text(encoding="utf-8")):
        raise ValueError("M4 corpus pointer differs from the verified harness pointer")
    expected_json, expected_md = render(harness, raw)
    expected_perf = update_perf(json.loads(expected_json))
    if (REPORT.read_text(encoding="utf-8") != expected_json or
            MARKDOWN.read_text(encoding="utf-8") != expected_md or
            PERF.read_text(encoding="utf-8") != expected_perf):
        raise ValueError("M4 report differs from retained trials or current corpus inputs")
    print(f"checked {len(raw['results'])} files / "
          f"{sum(len(row['trials']) for row in raw['results'])} fresh trials")


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    modes = parser.add_mutually_exclusive_group(required=True)
    modes.add_argument("--run", action="store_true")
    modes.add_argument("--check", action="store_true")
    args = parser.parse_args()
    harness = load_harness()
    try:
        run(harness) if args.run else check(harness)
    except (OSError, ValueError, KeyError, json.JSONDecodeError, RuntimeError) as exc:
        print(f"M4 corpus benchmark error: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
