#!/usr/bin/env python3
"""Measure the external real SVG corpus with the local macOS index harness."""

from __future__ import annotations

import argparse
import hashlib
import json
import math
import os
import platform
import re
import statistics
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CORPUS = Path("/Users/megasoft78/Desktop/Freelance/vexel-corpus-real")
RAW = ROOT / ".cache/M3-06/trials.json"
POINTER = ROOT / ".cache/M3-06/corpus-pointer.json"
REPORT = ROOT / "reports/M3/head-to-head.json"
MARKDOWN = ROOT / "reports/M3/head-to-head.md"
BIN = ROOT / "build/mac-release/bin/svg_index_bench"
TIMEOUT_SECONDS = 120


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def corpus_inputs(write_pointer: bool = False) -> tuple[list[dict], str, str]:
    manifest_path = CORPUS / "manifest.json"
    baseline_path = CORPUS / "INKSCAPE_BASELINE.md"
    manifest_hash = sha256(manifest_path)
    baseline_hash = sha256(baseline_path)
    entries = json.loads(manifest_path.read_text(encoding="utf-8"))
    baseline: dict[str, dict] = {}
    pattern = re.compile(r"^\| `(?P<path>[^`]+)` \| (?P<tier>T\d) \| (?P<size>\d+) \| (?P<status>[^|]+) \| (?P<seconds>[^|]+) \| (?P<rss>[^|]+) \| (?P<notes>[^|]+) \|$")
    for line in baseline_path.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            row = match.groupdict()
            baseline[row["path"]] = {
                "tier": row["tier"], "bytes": int(row["size"]),
                "status": row["status"].strip(),
                "seconds": float(row["seconds"].strip()),
                "peak_rss_bytes": int(row["rss"].strip()) if row["rss"].strip().isdigit() else None,
                "notes": row["notes"].strip(),
            }
    for item in entries:
        relative = Path(item["path"])
        file_path = (CORPUS / relative).resolve(strict=True)
        if not file_path.is_relative_to(CORPUS.resolve()) or not file_path.is_file():
            raise ValueError(f"invalid corpus path: {relative}")
        if file_path.stat().st_size != item["bytes"] or sha256(file_path) != item["sha256"]:
            raise ValueError(f"manifest size/hash mismatch: {relative}")
    pointer = {"corpus_root": str(CORPUS), "manifest_path": str(manifest_path),
               "manifest_sha256": manifest_hash, "baseline_path": str(baseline_path),
               "baseline_sha256": baseline_hash, "file_count": len(entries)}
    if write_pointer:
        POINTER.parent.mkdir(parents=True, exist_ok=True)
        POINTER.write_text(json.dumps(pointer, indent=2) + "\n", encoding="utf-8")
    return entries, manifest_hash, baseline_hash


def classify(result: subprocess.CompletedProcess[str], timed_out: bool) -> str:
    if timed_out or result.returncode is None or result.returncode < 0:
        return "crash"
    if result.returncode == 0:
        return "ok"
    try:
        measured = json.loads(result.stdout)
        if measured.get("outcome") == "unsupported":
            return "unsupported"
        if measured.get("outcome") == "limit":
            return "limit"
    except (json.JSONDecodeError, TypeError):
        pass
    message = result.stderr
    match = re.search(r"index failed: status=(\d+) diagnostic=(\d+)", message)
    if match:
        status = int(match.group(1))
        diagnostic = int(match.group(2))
        if status == 6:  # complete index but the current preview subset found no visible item
            return "unsupported"
        if status == 3 or diagnostic in (5, 6):  # allocation/storage or explicit input cap
            return "limit"
        if status in (1, 4, 5):
            return "unsupported"
    return "crash"


def run_one(path: Path) -> dict:
    started = time.perf_counter_ns()
    timed_out = False
    try:
        result = subprocess.run(["/usr/bin/time", "-l", str(BIN), str(path)],
                                capture_output=True, text=True, check=False,
                                timeout=TIMEOUT_SECONDS)
    except subprocess.TimeoutExpired as error:
        timed_out = True
        result = subprocess.CompletedProcess(error.cmd, -signal_number(),
                                             error.stdout or "", error.stderr or "")
    elapsed = time.perf_counter_ns() - started
    stdout = result.stdout if isinstance(result.stdout, str) else result.stdout.decode(errors="replace")
    stderr = result.stderr if isinstance(result.stderr, str) else result.stderr.decode(errors="replace")
    rss_match = re.search(r"^\s*(\d+)\s+maximum resident set size\s*$", stderr, re.MULTILINE)
    row = {"elapsed_ns": elapsed, "exit_code": result.returncode,
           "timed_out": timed_out, "timeout_seconds": TIMEOUT_SECONDS,
           "peak_rss_bytes": int(rss_match.group(1)) if rss_match else None,
           "diagnostic": stderr.strip()[-2000:],
           "outcome": classify(subprocess.CompletedProcess(result.args, result.returncode,
                                                           stdout, stderr), timed_out)}
    try:
        measured = json.loads(stdout)
        row.update({key: measured.get(key) for key in (
            "read_ns", "first_preview_ns", "complete_index_ns", "index_elapsed_ns", "max_step_ns",
            "index_complete", "status_code", "diagnostic_category", "diagnostic_offset",
            "preview_before_index_complete", "progress_10_50_90_complete",
            "element_count", "index_bytes", "peak_rss_bytes")})
        row["diagnostic"] = ""
        if row["outcome"] == "ok" and measured.get("outcome") != "ok":
            row["outcome"] = "crash"
    except (json.JSONDecodeError, TypeError):
        if row["outcome"] == "ok":
            row["outcome"] = "crash"
            row["diagnostic"] = (row["diagnostic"] + "\ninvalid benchmark JSON or missing metrics")[-2000:]
        row["stdout_excerpt"] = stdout.strip()[:1000]
    if row.get("peak_rss_bytes") is not None:
        row.pop("rss_unavailable_reason", None)
    else:
        row["rss_unavailable_reason"] = "macOS time -l emitted no maximum resident set size"
    if row["outcome"] != "ok":
        row["full_index_ns"] = None
        if row.get("complete_index_ns") is not None:
            row["full_index_ns"] = row["complete_index_ns"]
        missing = []
        if row.get("full_index_ns") is None:
            missing.append("full-index duration; index did not complete")
        if row.get("max_step_ns") is None:
            missing.append("main-thread max step; benchmark emitted no value")
        row["unavailable_metrics_reason"] = "; ".join(missing) if missing else None
    return row


def signal_number() -> int:
    import signal
    return -signal.SIGKILL


def run_campaign() -> dict:
    entries, manifest_hash, baseline_hash = corpus_inputs(write_pointer=True)
    if not BIN.is_file():
        raise ValueError("Release benchmark missing; run cmake --build --preset mac-release --target svg_index_bench")
    trials = []
    for item in entries:
        path = (CORPUS / item["path"]).resolve()
        repetitions = 3 if item["tier"] in ("T1", "T2") else 1
        record = {"id": item["id"], "path": str(path), "tier": item["tier"],
                  "bytes": item["bytes"], "sha256": item["sha256"], "trials": []}
        for _ in range(repetitions):
            record["trials"].append(run_one(path))
        trials.append(record)
        print(f"{item['id']}: {','.join(t['outcome'] for t in record['trials'])}", flush=True)
    raw = {"schema_version": 1, "environment": environment_metadata(),
           "manifest_path": str(CORPUS / "manifest.json"),
           "manifest_sha256": manifest_hash,
           "baseline_path": str(CORPUS / "INKSCAPE_BASELINE.md"),
           "baseline_sha256": baseline_hash,
           "timeout_seconds": TIMEOUT_SECONDS, "results": trials}
    RAW.parent.mkdir(parents=True, exist_ok=True)
    RAW.write_text(json.dumps(raw, indent=2) + "\n", encoding="utf-8")
    return raw


def geometric_mean(values: list[float]) -> float | None:
    if not values:
        return None
    return math.exp(sum(math.log(value) for value in values) / len(values))


def load_baselines(entries: list[dict], manifest_hash: str, baseline_hash: str) -> dict[str, dict]:
    baseline_doc = CORPUS / "INKSCAPE_BASELINE.md"
    baseline_rows: dict[str, dict] = {}
    pattern = re.compile(r"^\| `(?P<path>[^`]+)` \| (?P<tier>T\d) \| (?P<size>\d+) \| (?P<status>[^|]+) \| (?P<seconds>[^|]+) \| (?P<rss>[^|]+) \| (?P<notes>[^|]+) \|$")
    for line in baseline_doc.read_text(encoding="utf-8").splitlines():
        match = pattern.match(line)
        if match:
            row = match.groupdict()
            baseline_rows[row["path"]] = {
                "tier": row["tier"], "bytes": int(row["size"]),
                "status": row["status"].strip(), "seconds": float(row["seconds"].strip()),
                "peak_rss_bytes": int(row["rss"].strip()) if row["rss"].strip().isdigit() else None,
                "notes": row["notes"].strip(),
            }
    if len(baseline_rows) != len(entries):
        raise ValueError(f"expected one Inkscape baseline row per corpus file, found {len(baseline_rows)}")
    if {entry["path"] for entry in entries} != set(baseline_rows):
        raise ValueError("Inkscape baseline paths differ from manifest paths")
    for item in entries:
        baseline = baseline_rows[item["path"]]
        if baseline["tier"] != item["tier"] or baseline["bytes"] != item["bytes"]:
            raise ValueError(f"Inkscape baseline identity differs from manifest: {item['path']}")
    pointer = json.loads(POINTER.read_text(encoding="utf-8"))
    if (pointer.get("manifest_sha256") != manifest_hash or
            pointer.get("baseline_sha256") != baseline_hash or
            pointer.get("file_count") != len(entries)):
        raise ValueError("repo-local corpus pointer does not match current corpus inputs")
    return baseline_rows


def build_result_rows(raw: dict, entries: list[dict], baselines: dict[str, dict]) -> list[dict]:
    by_id = {entry["id"]: entry for entry in entries}
    result_rows = []
    for record in raw["results"]:
        item = by_id.get(record["id"])
        if item is None or item["sha256"] != record["sha256"] or len(record["trials"]) != (3 if item["tier"] in ("T1", "T2") else 1):
            raise ValueError(f"raw trial identity/repetition mismatch: {record['id']}")
        baseline = baselines.get(item["path"])
        trial_outcomes = [trial["outcome"] for trial in record["trials"]]
        overall = "ok" if all(value == "ok" for value in trial_outcomes) else next(
            (candidate for candidate in ("crash", "limit", "unsupported") if candidate in trial_outcomes), "crash")
        ok = [trial for trial in record["trials"] if trial["outcome"] == "ok"]
        metrics = {}
        for key in ("first_preview_ns", "complete_index_ns", "max_step_ns"):
            pool = ok if key == "first_preview_ns" else record["trials"]
            values = [trial[key] for trial in pool if trial.get(key) is not None]
            metrics[key] = int(statistics.median(values)) if values else None
        rss_values = [trial["peak_rss_bytes"] for trial in record["trials"]
                      if trial.get("peak_rss_bytes") is not None]
        metrics["peak_rss_bytes"] = int(statistics.median(rss_values)) if rss_values else None
        result_rows.append({"id": item["id"], "path": item["path"], "tier": item["tier"],
                            "bytes": item["bytes"], "sha256": item["sha256"],
                            "outcome": overall, "trial_outcomes": trial_outcomes,
                            "trials": record["trials"], "metrics": metrics,
                            "inkscape": baseline})
    if len(result_rows) != len(entries) or {row["id"] for row in result_rows} != set(by_id):
        raise ValueError("raw result count differs from manifest")
    return result_rows


def summarize_tiers(result_rows: list[dict]) -> dict[str, dict]:
    tier_summary = {}
    for tier in ("T1", "T2", "T3", "T4"):
        rows = [row for row in result_rows if row["tier"] == tier]
        speed, rss_ratio = [], []
        for row in rows:
            base = row["inkscape"]
            first = row["metrics"]["first_preview_ns"]
            rss = row["metrics"]["peak_rss_bytes"]
            if row["outcome"] == "ok" and base and base["status"].startswith("PASS") and first:
                speed.append(base["seconds"] / (first / 1e9))
                if base["peak_rss_bytes"] and rss:
                    rss_ratio.append(base["peak_rss_bytes"] / rss)
        tier_summary[tier] = {"files": len(rows), "ok": sum(r["outcome"] == "ok" for r in rows),
                              "unsupported": sum(r["outcome"] == "unsupported" for r in rows),
                              "limit": sum(r["outcome"] == "limit" for r in rows),
                              "crash": sum(r["outcome"] == "crash" for r in rows),
                              "comparable_successful_files": len(speed),
                              "inkscape_seconds_over_vexel_first_preview_geomean": geometric_mean(speed),
                              "inkscape_rss_over_vexel_rss_geomean": geometric_mean(rss_ratio),
                              "rss_comparable_files": len(rss_ratio)}
    return tier_summary


def wedge_summary(result_rows: list[dict]) -> dict:
    near_100_mib = [row for row in result_rows if row["tier"] == "T3" and
                    80 * 1024 * 1024 <= row["bytes"] <= 120 * 1024 * 1024]
    rss_checks = [{"id": row["id"], "source_bytes": row["bytes"],
                   "measured_peak_rss_bytes": row["metrics"]["peak_rss_bytes"],
                   "limit_bytes": row["bytes"] * 3 + 128 * 1024 * 1024,
                   "within_limit": row["metrics"]["peak_rss_bytes"] is not None and
                       row["metrics"]["peak_rss_bytes"] <= row["bytes"] * 3 + 128 * 1024 * 1024}
                  for row in near_100_mib]
    preview_ns = [row["metrics"]["first_preview_ns"] for row in near_100_mib
                  if row["metrics"]["first_preview_ns"] is not None]
    step_ns = [row["metrics"]["max_step_ns"] for row in near_100_mib
               if row["metrics"]["max_step_ns"] is not None]
    if len(preview_ns) == len(near_100_mib) and preview_ns:
        preview_status = "pass" if max(preview_ns) < 2000000000 else "fail_slow"
    elif not preview_ns and near_100_mib:
        preview_status = "fail_no_preview"
    else:
        preview_status = "incomplete"
    return {"first_visible_preview_100_mib": {"status": preview_status,
                "target_ns": 2000000000, "measured_files": len(preview_ns), "eligible_t3_files": len(near_100_mib),
                "max_first_preview_ns": max(preview_ns) if preview_ns else None,
                "preview_before_index_complete_files": sum(
                    1 for row in near_100_mib if any(t.get("preview_before_index_complete") for t in row["trials"]))},
                "maximum_main_thread_step": {"status": "pass" if len(step_ns) == len(near_100_mib) and step_ns and max(step_ns) < 100000000 else "fail_or_unavailable",
                "target_ns": 100000000, "measured_files": len(step_ns),
                "max_step_ns": max(step_ns) if step_ns else None},
                "peak_rss_100_mib": {"method": "measured peak RSS <= 3x source bytes + 128 MiB allowance, per WO-M3-01",
                "status": "pass" if rss_checks and all(x["within_limit"] for x in rss_checks) else "fail_or_unavailable",
                "files": rss_checks}}


def environment_metadata() -> dict:
    os_version = subprocess.run(["sw_vers", "-productVersion"], capture_output=True, text=True, check=False)
    os_name = subprocess.run(["sw_vers", "-productName"], capture_output=True, text=True, check=False)
    chip = subprocess.run(["sysctl", "-n", "machdep.cpu.brand_string"], capture_output=True, text=True, check=False)
    compiler = subprocess.run(["clang", "--version"], capture_output=True, text=True, check=False)
    return {"platform": platform.platform(), "architecture": platform.machine(),
            "python": sys.version.split()[0], "macos": f"{os_name.stdout.strip()} {os_version.stdout.strip()}".strip(),
            "chip": chip.stdout.strip() if chip.returncode == 0 else "unavailable",
            "compiler": compiler.stdout.splitlines()[0] if compiler.returncode == 0 else "unavailable",
            "binary_path": str(BIN), "binary_sha256": sha256(BIN), "build": "mac-release",
            "timeout_seconds": TIMEOUT_SECONDS,
            "process_policy": "fresh Vexel process per trial; /usr/bin/time -l captures RSS",
            "repetitions": {"T1-T2": 3, "T3-T4": 1}}


def aggregate(raw: dict) -> dict:
    entries, manifest_hash, baseline_hash = corpus_inputs()
    if raw["manifest_sha256"] != manifest_hash or raw["baseline_sha256"] != baseline_hash:
        raise ValueError("corpus manifest or Inkscape baseline changed since measurement")
    baselines = load_baselines(entries, manifest_hash, baseline_hash)
    rows = build_result_rows(raw, entries, baselines)
    environment = raw.get("environment")
    if not isinstance(environment, dict) or environment.get("binary_sha256") != sha256(BIN):
        raise ValueError("raw host/toolchain metadata missing or Release binary hash changed")
    return {"schema_version": 1, "corpus": {"root": str(CORPUS),
            "manifest_path": raw["manifest_path"], "manifest_sha256": raw["manifest_sha256"],
            "baseline_path": raw["baseline_path"], "baseline_sha256": raw["baseline_sha256"],
            "files": len(entries)}, "environment": environment,
            "comparison_limitations": ["Vexel open/index/first-supported-preview and Inkscape CLI probe are different tasks.",
                "This measures open/index behavior, not full-render parity.",
                "Time and RSS geometric means include only successful Vexel rows with passing Inkscape baseline rows."],
            "wedge_gates": wedge_summary(rows), "tier_summary": summarize_tiers(rows), "results": rows}


def render_markdown(report: dict) -> str:
    lines = ["# Real-corpus head-to-head", "",
             f"Corpus: `{report['corpus']['root']}` ({report['corpus']['files']} files), manifest SHA-256 `{report['corpus']['manifest_sha256']}`.",
             f"Baseline: Inkscape 1.4.4, SHA-256 `{report['corpus']['baseline_sha256']}`. Host: {report['environment']['macos']}, {report['environment']['chip']} / {report['environment']['architecture']}.",
             "", "This measures open/index and first supported preview. It does not measure full-render parity. Inkscape rows use the supplied CLI baseline protocol, which differs by tier and is not an identical task.",
             "", "## Per-file results", "",
             "| ID | Tier | Bytes | Outcome | Vexel first preview (ms) | Full index (ms) | Max step (ms) | Vexel peak RSS (bytes) | Inkscape seconds | Inkscape peak RSS (bytes) | Trials |",
             "|---|---:|---:|---|---:|---:|---:|---:|---:|---:|---|"]
    for row in report["results"]:
        m, base = row["metrics"], row["inkscape"]
        def ms(key: str) -> str:
            value = m[key]
            return "—" if value is None else f"{value / 1e6:.3f}"
        base_s = "—" if not base else f"{base['seconds']:.3f} ({base['status']})"
        base_rss = "—" if not base or base["peak_rss_bytes"] is None else str(base["peak_rss_bytes"])
        lines.append(f"| `{row['id']}` | {row['tier']} | {row['bytes']} | {row['outcome']} | {ms('first_preview_ns')} | {ms('complete_index_ns')} | {ms('max_step_ns')} | {m['peak_rss_bytes'] if m['peak_rss_bytes'] is not None else '—'} | {base_s} | {base_rss} | {','.join(row['trial_outcomes'])} |")
    lines += ["", "## Tier summary", "", "Speedup is Inkscape supplied probe seconds divided by Vexel first-preview seconds; RSS ratio is Inkscape peak RSS divided by Vexel peak RSS. Both geometric means include only successful Vexel outcomes and PASS Inkscape rows. Protocols differ, so these are directional workload comparisons.",
              "", "| Tier | Files | OK | Unsupported | Limit | Crash/timeout | Comparable files | Geomean time ratio | Geomean RSS ratio |", "|---|---:|---:|---:|---:|---:|---:|---:|---:|"]
    for tier, summary in report["tier_summary"].items():
        speed = summary["inkscape_seconds_over_vexel_first_preview_geomean"]
        rss = summary["inkscape_rss_over_vexel_rss_geomean"]
        lines.append(f"| {tier} | {summary['files']} | {summary['ok']} | {summary['unsupported']} | {summary['limit']} | {summary['crash']} | {summary['comparable_successful_files']} | {speed:.3f}x | {rss:.3f}x ({summary['rss_comparable_files']} files) |" if speed is not None and rss is not None else f"| {tier} | {summary['files']} | {summary['ok']} | {summary['unsupported']} | {summary['limit']} | {summary['crash']} | {summary['comparable_successful_files']} | {'—' if speed is None else f'{speed:.3f}x'} | {'—' if rss is None else f'{rss:.3f}x'} ({summary['rss_comparable_files']} files) |")
    wedge = report["wedge_gates"]
    rss_gate = wedge["peak_rss_100_mib"]
    lines += ["", "## Wedge checks", "", f"- 100 MiB first visible preview under 2 seconds: **{wedge['first_visible_preview_100_mib']['status']}**; measured {wedge['first_visible_preview_100_mib']['measured_files']} of {wedge['first_visible_preview_100_mib']['eligible_t3_files']} T3 files in the 80–120 MiB band; maximum {wedge['first_visible_preview_100_mib']['max_first_preview_ns']} ns.",
              f"- Maximum main-thread index step under 100 ms: **{wedge['maximum_main_thread_step']['status']}**; maximum {wedge['maximum_main_thread_step']['max_step_ns']} ns. Pre-index preview occurred in {wedge['first_visible_preview_100_mib']['preview_before_index_complete_files']} eligible files.",
              f"- 100 MiB-band RSS allowance (3× source bytes + 128 MiB): **{rss_gate['status']}** across {len(rss_gate['files'])} eligible files; measured peak RSS appears in the JSON and per-file table.",
              "- Inkscape RSS can exceed Vexel's indexed input footprint substantially; this comparison is not a memory parity claim.",
              "", "## Failures and limits", ""]
    failed = [row for row in report["results"] if row["outcome"] != "ok"]
    if not failed:
        lines.append("No non-OK outcomes.")
    for row in failed:
        reasons = []
        for trial in row["trials"]:
            if trial["outcome"] == "ok":
                continue
            reason = (f"status={trial.get('status_code')} diagnostic={trial.get('diagnostic_category')}"
                      f"@{trial.get('diagnostic_offset')} exit={trial['exit_code']}")
            if trial.get("timed_out"):
                reason += f" timed out after {trial['timeout_seconds']}s"
            reasons.append(reason)
        lines.append(f"- `{row['id']}`: {row['outcome']} ({'; '.join(reasons) or 'see JSON trial diagnostics'}).")
    lines += ["", "## Reproducibility", "", f"Release binary SHA-256: `{report['environment']['binary_sha256']}`. Fresh processes, 3 repetitions for T1/T2, 1 for T3/T4, each with a {TIMEOUT_SECONDS}s timeout. External source files remain at the referenced corpus path; no corpus file is stored in this repository.", ""]
    return "\n".join(lines)


def check() -> None:
    raw = json.loads(RAW.read_text(encoding="utf-8"))
    expected = aggregate(raw)
    expected_json = json.dumps(expected, indent=2) + "\n"
    expected_md = render_markdown(expected)
    if REPORT.read_text(encoding="utf-8") != expected_json or MARKDOWN.read_text(encoding="utf-8") != expected_md:
        raise ValueError("head-to-head report differs from retained raw trials or current inputs")
    print(f"checked {len(expected['results'])} files and {sum(len(x['trials']) for x in expected['results'])} trials")


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--run", action="store_true")
    mode.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.run:
        raw = run_campaign()
        report = aggregate(raw)
        REPORT.write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
        MARKDOWN.write_text(render_markdown(report), encoding="utf-8")
        check()
    else:
        check()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, json.JSONDecodeError) as error:
        print(f"head-to-head error: {error}", file=sys.stderr)
        raise SystemExit(1)
