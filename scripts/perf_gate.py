#!/usr/bin/env python3
"""Strict local macOS performance report and repeated-violation gate."""
from __future__ import annotations

import argparse
import hashlib
import json
import math
import platform
import statistics
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
BASELINE_PATH = ROOT / "bench/results/perf-gate-baseline.json"
M301_REPORT = ROOT / "bench/results/WO-M3-01.json"
METRICS = {
    "10MiB.first_preview_p95_ns": ("10MiB", "first_preview_p95_ns"),
    "100MiB.first_preview_p95_ns": ("100MiB", "first_preview_p95_ns"),
    "100MiB.max_step_ns": ("100MiB", "max_step_ns"),
    "100MiB.peak_rss_bytes": ("100MiB", "peak_rss_bytes"),
}
STRETCH = {
    "cached_pan_zoom_p95_ns": 33_000_000,
    "single_arch_stripped_executable_bytes": 10 * 1024 * 1024,
    "app_bundle_bytes": 25 * 1024 * 1024,
}


class GateError(ValueError):
    pass


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(block)
    return digest.hexdigest()


def exact_keys(obj: Any, expected: set[str], label: str) -> dict[str, Any]:
    if not isinstance(obj, dict):
        raise GateError(f"{label} must be an object")
    missing, extra = expected - set(obj), set(obj) - expected
    if missing or extra:
        raise GateError(f"{label} fields invalid (missing={sorted(missing)}, unknown={sorted(extra)})")
    return obj


def strict_string(value: Any, label: str) -> str:
    if not isinstance(value, str) or not value:
        raise GateError(f"{label} must be a non-empty string")
    return value


def strict_hash(value: Any, label: str) -> str:
    if not isinstance(value, str) or len(value) != 64 or any(c not in "0123456789abcdef" for c in value):
        raise GateError(f"{label} must be a lowercase SHA-256")
    return value


def strict_positive_int(value: Any, label: str) -> int:
    if type(value) is not int or value <= 0 or value > (1 << 63) - 1:
        raise GateError(f"{label} must be a positive 64-bit integer")
    return value


def strict_bool(value: Any, label: str) -> bool:
    if type(value) is not bool:
        raise GateError(f"{label} must be a boolean")
    return value


def finite_nonnegative(value: Any, label: str) -> float:
    if type(value) is int:
        if value < 0 or value > (1 << 63) - 1:
            raise GateError(f"{label} must be a finite non-negative 64-bit value")
        return value
    if type(value) is not float or not math.isfinite(value) or value < 0:
        raise GateError(f"{label} must be a finite non-negative number")
    return value


def load_json(path: Path, label: str) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"), parse_constant=lambda v: (_ for _ in ()).throw(GateError(f"{label} contains non-finite {v}")))
    except (OSError, json.JSONDecodeError) as exc:
        raise GateError(f"cannot read {label}: {exc}") from exc


def validate_baseline(value: Any) -> dict[str, Any]:
    exact_keys(value, {"schema_version", "source_work_order", "source_commit", "source_report", "source_report_sha256", "host_toolchain", "limits", "regression_tolerances", "cases", "stretch_targets"}, "baseline")
    if type(value["schema_version"]) is not int or value["schema_version"] != 1 or value["source_work_order"] != "WO-M3-01":
        raise GateError("baseline schema or provenance is invalid")
    for name in ("source_commit", "source_report_sha256"):
        if not isinstance(value[name], str) or not value[name]:
            raise GateError(f"baseline {name} is invalid")
    exact_keys(value["host_toolchain"], {"machine_model", "chip", "memory_bytes", "architecture", "os", "compiler"}, "baseline host_toolchain")
    exact_keys(value["limits"], set(METRICS), "baseline limits")
    exact_keys(value["regression_tolerances"], set(METRICS), "baseline regression_tolerances")
    exact_keys(value["cases"], {"10MiB", "100MiB"}, "baseline cases")
    for case in ("10MiB", "100MiB"):
        exact_keys(value["cases"][case], {"input_sha256", "first_preview_median_ns", "first_preview_p95_ns", "max_step_ns", "peak_rss_bytes", "index_bytes"}, f"baseline {case}")
        if not isinstance(value["cases"][case]["input_sha256"], str) or len(value["cases"][case]["input_sha256"]) != 64:
            raise GateError(f"baseline {case} input hash is invalid")
        for key, item in value["cases"][case].items():
            if key != "input_sha256":
                finite_nonnegative(item, f"baseline {case}.{key}")
    exact_keys(value["stretch_targets"], set(STRETCH), "baseline stretch_targets")
    for key, item in value["limits"].items():
        finite_nonnegative(item, f"baseline limits.{key}")
    for key, item in value["regression_tolerances"].items():
        if type(item) is int:
            valid = 0 <= item <= 1
        else:
            valid = type(item) is float and math.isfinite(item) and 0 <= item <= 1
        if not valid:
            raise GateError(f"baseline regression tolerance {key} must be finite and from 0 to 1")
    for key, item in value["stretch_targets"].items():
        finite_nonnegative(item, f"baseline stretch_targets.{key}")
    report_path = ROOT / value["source_report"]
    if not report_path.is_file() or sha256(report_path) != value["source_report_sha256"]:
        raise GateError("pinned M3-01 report is missing or its SHA-256 changed")
    if value["source_commit"] != "159c7e761ce87862e85742cd25fb61a9aef01da4" or value["source_report"] != "bench/results/WO-M3-01.json":
        raise GateError("baseline source commit or report path is not the frozen M3-01 source")
    source = load_json(report_path, "pinned M3-01 report")
    expected_host = {
        "machine_model": source["machine_model"], "chip": source["chip"],
        "memory_bytes": source["memory_bytes"], "architecture": source["architecture"],
        "os": "macOS " + source["machine"].split("macOS-", 1)[1].split("-", 1)[0],
        "compiler": source["compiler"],
    }
    if value["host_toolchain"] != expected_host:
        raise GateError("baseline host/toolchain fields do not match the pinned M3-01 report")
    source_fields = {
        "input_sha256": "source_sha256", "first_preview_median_ns": "first_preview_median_ns",
        "first_preview_p95_ns": "first_preview_p95_ns", "max_step_ns": "max_step_latency_ns",
        "peak_rss_bytes": "peak_child_rss_bytes", "index_bytes": "index_bytes",
    }
    for case in ("10MiB", "100MiB"):
        for target, field in source_fields.items():
            if value["cases"][case][target] != source["cases"][case][field]:
                raise GateError(f"baseline {case}.{target} does not match the pinned M3-01 report")
    return value


def validate_history(history: Any) -> dict[str, Any]:
    exact_keys(history, {"schema_version", "metrics"}, "history")
    if type(history["schema_version"]) is not int or history["schema_version"] != 1:
        raise GateError("history schema_version must be 1")
    exact_keys(history["metrics"], set(METRICS), "history.metrics")
    for metric, record in history["metrics"].items():
        exact_keys(record, {"streak", "last_status", "last_value"}, f"history.metrics.{metric}")
        if type(record["streak"]) is not int or record["streak"] not in (0, 1, 2):
            raise GateError(f"history {metric}.streak must be 0, 1, or 2")
        if record["last_status"] not in ("none", "healthy", "violation"):
            raise GateError(f"history {metric}.last_status is invalid")
        if ((record["streak"] == 0 and record["last_status"] == "violation") or
                (record["streak"] > 0 and record["last_status"] != "violation")):
            raise GateError(f"history {metric} streak/status combination is invalid")
        if record["last_value"] is not None:
            finite_nonnegative(record["last_value"], f"history {metric}.last_value")
    return history


def empty_history() -> dict[str, Any]:
    return {"schema_version": 1, "metrics": {name: {"streak": 0, "last_status": "none", "last_value": None} for name in METRICS}}


def evaluate(metrics: dict[str, float | None], limits: dict[str, float], baselines: dict[str, float], tolerances: dict[str, float], history: dict[str, Any], comparable: bool, noisy: set[str] | None = None) -> tuple[dict[str, Any], dict[str, Any]]:
    validate_history(history)
    noisy = noisy or set()
    if any(set(values) != set(METRICS) for values in (metrics, limits, baselines, tolerances)):
        raise GateError("metric set does not match the frozen gate contract")
    results: dict[str, Any] = {}
    updated = json.loads(json.dumps(history))
    if not comparable:
        for metric, value in metrics.items():
            if value is not None:
                finite_nonnegative(value, metric)
            results[metric] = {"value": value, "absolute_limit": limits[metric], "baseline_value": baselines[metric], "status": "incomparable", "consecutive_violations": history["metrics"][metric]["streak"]}
        return results, updated
    for metric, value in metrics.items():
        finite_nonnegative(limits[metric], f"limit {metric}")
        finite_nonnegative(baselines[metric], f"baseline {metric}")
        tolerance = tolerances[metric]
        if type(tolerance) is int:
            valid_tolerance = 0 <= tolerance <= 1
        else:
            valid_tolerance = type(tolerance) is float and math.isfinite(tolerance) and 0 <= tolerance <= 1
        if not valid_tolerance:
            raise GateError(f"tolerance {metric} must be finite and from 0 to 1")
        record = updated["metrics"][metric]
        if value is None:
            results[metric] = {"value": None, "absolute_limit": limits[metric], "baseline_value": baselines[metric], "status": "unavailable", "reason": "required measurement is unavailable", "consecutive_violations": record["streak"]}
            continue
        finite_nonnegative(value, metric)
        if metric in noisy:
            results[metric] = {"value": value, "absolute_limit": limits[metric], "baseline_value": baselines[metric], "status": "noisy", "reason": "trial coefficient of variation exceeds 10 percent", "consecutive_violations": record["streak"]}
            continue
        regression_limit = baselines[metric] * (1.0 + tolerance)
        effective_limit = min(limits[metric], regression_limit)
        if value > effective_limit:
            record["streak"] = min(2, record["streak"] + 1)
            record["last_status"] = "violation"
            record["last_value"] = value
            status = "fail" if record["streak"] >= 2 else "warning"
        else:
            record.update(streak=0, last_status="healthy", last_value=value)
            status = "pass"
        results[metric] = {"value": value, "absolute_limit": limits[metric], "regression_limit": regression_limit, "effective_limit": effective_limit, "baseline_value": baselines[metric], "tolerance_fraction": tolerance, "status": status, "consecutive_violations": record["streak"]}
    return results, updated


def identity_from_hardware(hardware: dict[str, Any]) -> dict[str, Any]:
    return {key: hardware[key] for key in ("machine_model", "chip", "memory_bytes", "architecture", "os", "compiler")}


def comparison_reason(baseline: dict[str, Any], hardware: dict[str, Any], cases: dict[str, Any]) -> list[str]:
    reasons = []
    if identity_from_hardware(hardware) != baseline["host_toolchain"]:
        reasons.append("host/toolchain identity differs from pinned M3-01 baseline")
    for case in ("10MiB", "100MiB"):
        if cases[case]["input_sha256"] != baseline["cases"][case]["input_sha256"]:
            reasons.append(f"{case} workload input hash differs from pinned baseline")
    return reasons


def percentile(values: list[int], fraction: float) -> int:
    ordered = sorted(values)
    return ordered[max(1, math.ceil(len(ordered) * fraction)) - 1]


def variability(values: list[int]) -> dict[str, float]:
    mean = statistics.fmean(values)
    deviation = statistics.stdev(values) if len(values) > 1 else 0.0
    return {"mean": mean, "sample_standard_deviation": deviation,
            "coefficient_of_variation": deviation / mean if mean else 0.0}


def run_checked(argv: list[str]) -> str:
    completed = subprocess.run(argv, cwd=ROOT, text=True, capture_output=True, check=False)
    if completed.returncode:
        raise GateError(f"command failed ({completed.returncode}): {' '.join(argv)}\n{completed.stderr.strip()}")
    return completed.stdout


def hardware_now() -> dict[str, Any]:
    from bench.generate.index_bench import hardware_metadata
    return hardware_metadata()


def validate_raw(raw: Any) -> dict[str, Any]:
    exact_keys(raw, {"schema_version", "seed", "machine", "machine_model", "chip", "memory_bytes", "architecture", "os", "compiler", "build", "cache_caveat", "cases"}, "raw observations")
    if type(raw["schema_version"]) is not int or raw["schema_version"] != 1:
        raise GateError("raw schema_version must be integer 1")
    if type(raw["seed"]) is not int or raw["seed"] != 20261009:
        raise GateError("raw seed does not match frozen campaign")
    for name in ("machine", "machine_model", "chip", "architecture", "os", "compiler", "cache_caveat"):
        strict_string(raw[name], f"raw {name}")
    strict_positive_int(raw["memory_bytes"], "raw memory_bytes")
    if raw["build"] != "mac-release":
        raise GateError("raw build must be mac-release")
    cases = exact_keys(raw["cases"], {"10MiB", "100MiB"}, "raw cases")
    case_keys = {"source_bytes", "source_sha256", "workload", "cold_trials", "warm_trials", "source_read_median_ns", "source_read_p95_ns", "first_preview_median_ns", "first_preview_p95_ns", "complete_index_median_ns", "max_step_latency_ns", "progressive_replay", "peak_child_rss_bytes", "index_bytes"}
    trial_keys = {"source_bytes", "read_ns", "first_preview_ns", "complete_index_ns", "max_step_ns", "preview_before_index_complete", "progress_10_50_90_complete", "element_count", "index_bytes", "peak_rss_bytes", "process_elapsed_ns"}
    for label, data in cases.items():
        exact_keys(data, case_keys, f"raw {label}")
        strict_positive_int(data["source_bytes"], f"raw {label}.source_bytes")
        strict_hash(data["source_sha256"], f"raw {label}.source_sha256")
        strict_string(data["workload"], f"raw {label}.workload")
        replay = exact_keys(data["progressive_replay"], {"events", "first_preview_before_index_complete", "worker_render_completed"}, f"raw {label}.progressive_replay")
        if replay["events"] != [10, 50, 90, 100]:
            raise GateError(f"raw {label}.progressive_replay.events is invalid")
        strict_bool(replay["first_preview_before_index_complete"], f"raw {label}.progressive_replay.first_preview_before_index_complete")
        strict_bool(replay["worker_render_completed"], f"raw {label}.progressive_replay.worker_render_completed")
        for field in ("source_read_median_ns", "source_read_p95_ns", "first_preview_median_ns", "first_preview_p95_ns", "complete_index_median_ns", "max_step_latency_ns", "peak_child_rss_bytes", "index_bytes"):
            finite_nonnegative(data[field], f"raw {label}.{field}")
        for group, count in (("cold_trials", 5), ("warm_trials", 10)):
            trials = data[group]
            if not isinstance(trials, list) or len(trials) != count:
                raise GateError(f"raw {label}.{group} must contain exactly {count} trials")
            for idx, trial in enumerate(trials):
                exact_keys(trial, trial_keys, f"raw {label}.{group}[{idx}]")
                for key in trial_keys - {"preview_before_index_complete", "progress_10_50_90_complete"}:
                    finite_nonnegative(trial[key], f"raw {label}.{group}[{idx}].{key}")
                strict_bool(trial["preview_before_index_complete"], f"raw {label}.{group}[{idx}].preview_before_index_complete")
                strict_bool(trial["progress_10_50_90_complete"], f"raw {label}.{group}[{idx}].progress_10_50_90_complete")
    return raw


def validate_result_build(result: dict[str, Any]) -> None:
    build = exact_keys(result["build"], {"configuration", "executable", "executable_sha256", "executable_bytes", "benchmark_executable", "benchmark_executable_sha256", "benchmark_executable_bytes", "dependencies", "source_commit", "working_tree_dirty", "source_diff_sha256"}, "result build")
    for name in ("configuration", "executable", "benchmark_executable"):
        strict_string(build[name], f"result build.{name}")
    for name in ("executable_sha256", "benchmark_executable_sha256", "source_diff_sha256"):
        strict_hash(build[name], f"result build.{name}")
    for name in ("executable_bytes", "benchmark_executable_bytes"):
        strict_positive_int(build[name], f"result build.{name}")
    strict_bool(build["working_tree_dirty"], "result build.working_tree_dirty")
    if build["source_commit"] != result["build_sha"]:
        raise GateError("result source_commit and build_sha differ")
    if not isinstance(build["dependencies"], list):
        raise GateError("result build.dependencies must be a list")
    for idx, item in enumerate(build["dependencies"]):
        item = exact_keys(item, {"install_name", "identity"}, f"result dependency {idx}")
        strict_string(item["install_name"], f"result dependency {idx}.install_name")
        strict_string(item["identity"], f"result dependency {idx}.identity")


def validate_result_case(label: str, case: Any) -> None:
    exact_keys(case, {"input_bytes", "input_sha256", "workload", "cold_order_trials", "warm_order_trials", "summary", "trial_variability"}, f"result {label}")
    strict_positive_int(case["input_bytes"], f"result {label}.input_bytes")
    strict_hash(case["input_sha256"], f"result {label}.input_sha256")
    strict_string(case["workload"], f"result {label}.workload")
    trial_keys = {"source_bytes", "read_ns", "first_preview_ns", "complete_index_ns", "max_step_ns", "preview_before_index_complete", "progress_10_50_90_complete", "element_count", "index_bytes", "peak_rss_bytes", "process_elapsed_ns"}
    for group, count in (("cold_order_trials", 5), ("warm_order_trials", 10)):
        if not isinstance(case[group], list) or len(case[group]) != count:
            raise GateError(f"result {label}.{group} must contain {count} trials")
        for idx, trial in enumerate(case[group]):
            exact_keys(trial, trial_keys, f"result {label}.{group}[{idx}]")
            for key, value in trial.items():
                if key in ("preview_before_index_complete", "progress_10_50_90_complete"):
                    strict_bool(value, f"result {label}.{group}[{idx}].{key}")
                else:
                    finite_nonnegative(value, f"result {label}.{group}[{idx}].{key}")
    summary = exact_keys(case["summary"], {"source_read_median_ns", "source_read_p95_ns", "first_preview_median_ns", "first_preview_p95_ns", "complete_index_median_ns", "complete_index_p95_ns", "max_step_ns", "peak_rss_bytes", "index_bytes"}, f"result {label}.summary")
    for key, value in summary.items():
        finite_nonnegative(value, f"result {label}.summary.{key}")
    variability = exact_keys(case["trial_variability"], {"first_preview_ns", "max_step_ns", "peak_rss_bytes"}, f"result {label}.trial_variability")
    for key, stats in variability.items():
        stats = exact_keys(stats, {"mean", "sample_standard_deviation", "coefficient_of_variation"}, f"result {label}.trial_variability.{key}")
        for stat, value in stats.items():
            finite_nonnegative(value, f"result {label}.trial_variability.{key}.{stat}")


def validate_result_gate(gate: Any) -> None:
    gate = exact_keys(gate, {"status", "metrics", "absolute_limits", "regression_tolerances"}, "result gate")
    if gate["status"] not in ("incomparable", "fail", "warning", "noisy", "pass"):
        raise GateError("result gate status is invalid")
    for field in ("metrics", "absolute_limits", "regression_tolerances"):
        exact_keys(gate[field], set(METRICS), f"result gate.{field}")
    for metric, item in gate["metrics"].items():
        if not isinstance(item, dict) or item.get("status") not in ("incomparable", "unavailable", "noisy", "pass", "warning", "fail"):
            raise GateError(f"result gate metric {metric} is malformed")
        validate_gate_metric(metric, item)
    for metric, value in gate["absolute_limits"].items():
        finite_nonnegative(value, f"result gate.absolute_limits.{metric}")
    for metric, value in gate["regression_tolerances"].items():
        if type(value) not in (int, float) or not math.isfinite(value) or not 0 <= value <= 1:
            raise GateError(f"result gate.regression_tolerances.{metric} is invalid")


def validate_gate_metric(metric: str, item: dict[str, Any]) -> None:
    status = item["status"]
    expected = {"value", "absolute_limit", "baseline_value", "status", "consecutive_violations", "delta_from_baseline"}
    if status in ("pass", "warning", "fail"):
        expected |= {"regression_limit", "effective_limit", "tolerance_fraction"}
    if status in ("unavailable", "noisy"):
        expected.add("reason")
    exact_keys(item, expected, f"result gate metric {metric}")
    for key in ("value", "delta_from_baseline"):
        if item[key] is not None:
            finite_nonnegative(item[key], f"result gate metric {metric}.{key}")
    for key in expected - {"status", "reason", "value", "consecutive_violations", "delta_from_baseline"}:
        finite_nonnegative(item[key], f"result gate metric {metric}.{key}")
    if type(item["consecutive_violations"]) is not int or item["consecutive_violations"] not in (0, 1, 2):
        raise GateError(f"result gate metric {metric}.consecutive_violations is invalid")
    if "reason" in item:
        strict_string(item["reason"], f"result gate metric {metric}.reason")


def validate_result_stretch(stretch: Any) -> None:
    stretch = exact_keys(stretch, {"targets", "executable_bytes", "executable_status", "executable_reason", "app_bundle", "pan_zoom_event_to_present_p95_ns"}, "result stretch_targets")
    targets = exact_keys(stretch["targets"], set(STRETCH), "result stretch_targets.targets")
    for key, value in targets.items():
        finite_nonnegative(value, f"result stretch_targets.targets.{key}")
    finite_nonnegative(stretch["executable_bytes"], "result stretch_targets.executable_bytes")
    strict_string(stretch["executable_status"], "result stretch_targets.executable_status")
    strict_string(stretch["executable_reason"], "result stretch_targets.executable_reason")
    bundle = exact_keys(stretch["app_bundle"], {"bytes", "status", "reason"}, "result stretch_targets.app_bundle")
    if bundle["bytes"] is not None:
        finite_nonnegative(bundle["bytes"], "result stretch_targets.app_bundle.bytes")
    strict_string(bundle["status"], "result stretch_targets.app_bundle.status")
    strict_string(bundle["reason"], "result stretch_targets.app_bundle.reason")
    pan = exact_keys(stretch["pan_zoom_event_to_present_p95_ns"], {"value", "status", "reason"}, "result stretch_targets.pan_zoom_event_to_present_p95_ns")
    if pan["value"] is not None:
        finite_nonnegative(pan["value"], "result stretch_targets.pan_zoom_event_to_present_p95_ns.value")
    strict_string(pan["status"], "result stretch_targets.pan_zoom_event_to_present_p95_ns.status")
    strict_string(pan["reason"], "result stretch_targets.pan_zoom_event_to_present_p95_ns.reason")


def validate_result(result: Any) -> dict[str, Any]:
    """Reject unknown and malformed nested fields before a report is retained."""
    exact_keys(result, {"schema_version", "run_timestamp_utc", "build_sha", "build", "hardware_toolchain", "seed", "trial_protocol", "cases", "baseline_provenance", "comparison", "gate", "stretch_targets"}, "result")
    if type(result["schema_version"]) is not int or result["schema_version"] != 1:
        raise GateError("result schema_version must be integer 1")
    strict_string(result["run_timestamp_utc"], "result run_timestamp_utc")
    if not isinstance(result["build_sha"], str) or len(result["build_sha"]) != 40 or any(c not in "0123456789abcdef" for c in result["build_sha"]):
        raise GateError("result build_sha must be a lowercase git SHA-1")
    if type(result["seed"]) is not int or result["seed"] != 20261009:
        raise GateError("result seed does not match frozen campaign")
    validate_result_build(result)
    hardware = exact_keys(result["hardware_toolchain"], {"machine", "machine_model", "chip", "memory_bytes", "architecture", "os", "compiler"}, "result hardware_toolchain")
    for name, value in hardware.items():
        strict_positive_int(value, f"result hardware_toolchain.{name}") if name == "memory_bytes" else strict_string(value, f"result hardware_toolchain.{name}")
    protocol = exact_keys(result["trial_protocol"], {"cold_order_trials", "warm_order_trials", "cache_caveat"}, "result trial_protocol")
    if type(protocol["cold_order_trials"]) is not int or protocol["cold_order_trials"] != 5 or type(protocol["warm_order_trials"]) is not int or protocol["warm_order_trials"] != 10:
        raise GateError("result trial protocol does not match frozen campaign")
    strict_string(protocol["cache_caveat"], "result trial_protocol.cache_caveat")
    cases = exact_keys(result["cases"], {"10MiB", "100MiB"}, "result cases")
    for label, case in cases.items():
        validate_result_case(label, case)
    exact_keys(result["baseline_provenance"], {"work_order", "commit", "report", "report_sha256"}, "result baseline_provenance")
    strict_hash(result["baseline_provenance"]["report_sha256"], "result baseline_provenance.report_sha256")
    comparison = exact_keys(result["comparison"], {"status", "reasons", "baseline_values"}, "result comparison")
    if comparison["status"] not in ("comparable", "incomparable") or not isinstance(comparison["reasons"], list) or any(not isinstance(x, str) for x in comparison["reasons"]):
        raise GateError("result comparison is malformed")
    exact_keys(comparison["baseline_values"], {"10MiB", "100MiB"}, "result comparison.baseline_values")
    for label, case in comparison["baseline_values"].items():
        exact_keys(case, {"input_sha256", "first_preview_median_ns", "first_preview_p95_ns", "max_step_ns", "peak_rss_bytes", "index_bytes"}, f"result comparison.baseline_values.{label}")
        strict_hash(case["input_sha256"], f"result comparison.baseline_values.{label}.input_sha256")
        for key, value in case.items():
            if key != "input_sha256":
                finite_nonnegative(value, f"result comparison.baseline_values.{label}.{key}")
    validate_result_gate(result["gate"])
    validate_result_stretch(result["stretch_targets"])
    return result

def under_cache(path: Path) -> Path:
    resolved = (ROOT / path).resolve() if not path.is_absolute() else path.resolve()
    cache = (ROOT / ".cache").resolve()
    if resolved != cache and cache not in resolved.parents:
        raise GateError("result and history paths must be under repository .cache/")
    return resolved


def source_metadata() -> tuple[str, bool, str]:
    try:
        commit = run_checked(["git", "rev-parse", "HEAD"]).strip()
        dirty = bool(run_checked(["git", "status", "--porcelain"]).strip())
        diff = subprocess.run(["git", "diff", "HEAD", "--binary"], cwd=ROOT,
                              capture_output=True, check=True).stdout
        return commit, dirty, hashlib.sha256(diff).hexdigest()
    except (GateError, subprocess.CalledProcessError):
        return "unknown", True, "unavailable"


def run_raw_campaign() -> dict[str, Any]:
    run_checked(["cmake", "--preset", "mac-release"])
    run_checked(["cmake", "--build", "--preset", "mac-release", "--target", "svg_index_bench", "svgworkbench"])
    raw_path = ROOT / ".cache/perf-gate/raw-observations.json"
    raw_path.parent.mkdir(parents=True, exist_ok=True)
    run_checked([sys.executable, "bench/generate/index_bench.py", "--sizes", "10MiB,100MiB", "--cold-trials", "5", "--warm-trials", "10", "--seed", "20261009", "--output", str(raw_path.relative_to(ROOT))])
    return validate_raw(load_json(raw_path, "raw benchmark observations"))


def summarize_case(label: str, data: dict[str, Any]) -> tuple[dict[str, Any], set[str]]:
    trials = data["cold_trials"] + data["warm_trials"]
    summary = {
        "source_read_median_ns": percentile([t["read_ns"] for t in trials], .5),
        "source_read_p95_ns": percentile([t["read_ns"] for t in trials], .95),
        "first_preview_median_ns": percentile([t["first_preview_ns"] for t in trials], .5),
        "first_preview_p95_ns": percentile([t["first_preview_ns"] for t in trials], .95),
        "complete_index_median_ns": percentile([t["complete_index_ns"] for t in trials], .5),
        "complete_index_p95_ns": percentile([t["complete_index_ns"] for t in trials], .95),
        "max_step_ns": max(t["max_step_ns"] for t in trials),
        "peak_rss_bytes": max(t["peak_rss_bytes"] for t in trials),
        "index_bytes": max(t["index_bytes"] for t in trials),
    }
    keys = ("first_preview_ns", "max_step_ns", "peak_rss_bytes")
    stats = {key: variability([t[key] for t in trials]) for key in keys}
    metric_keys = {"10MiB": ("10MiB.first_preview_p95_ns",),
                   "100MiB": ("100MiB.first_preview_p95_ns", "100MiB.max_step_ns", "100MiB.peak_rss_bytes")}
    trial_keys = {"10MiB": ("first_preview_ns",),
                  "100MiB": ("first_preview_ns", "max_step_ns", "peak_rss_bytes")}
    noisy = {metric for metric, trial_key in zip(metric_keys[label], trial_keys[label])
             if stats[trial_key]["coefficient_of_variation"] > 0.10}
    result = {"input_bytes": data["source_bytes"], "input_sha256": data["source_sha256"],
              "workload": data["workload"], "cold_order_trials": data["cold_trials"],
              "warm_order_trials": data["warm_trials"], "summary": summary,
              "trial_variability": stats}
    return result, noisy


def summarize_cases(raw: dict[str, Any]) -> tuple[dict[str, Any], set[str]]:
    cases: dict[str, Any] = {}
    noisy: set[str] = set()
    for label, data in raw["cases"].items():
        cases[label], case_noisy = summarize_case(label, data)
        noisy.update(case_noisy)
    return cases, noisy


def gate_report(baseline: dict[str, Any], cases: dict[str, Any], hardware: dict[str, Any], history_path: Path, noisy: set[str]) -> tuple[dict[str, Any], dict[str, Any], bool, list[str], str]:
    reasons = comparison_reason(baseline, hardware, cases)
    history = validate_history(load_json(history_path, "history")) if history_path.exists() else empty_history()
    fields = {"10MiB.first_preview_p95_ns": ("10MiB", "first_preview_p95_ns"),
              "100MiB.first_preview_p95_ns": ("100MiB", "first_preview_p95_ns"),
              "100MiB.max_step_ns": ("100MiB", "max_step_ns"),
              "100MiB.peak_rss_bytes": ("100MiB", "peak_rss_bytes")}
    metrics = {name: cases[case]["summary"][field] for name, (case, field) in fields.items()}
    baseline_values = {name: baseline["cases"][case][field] for name, (case, field) in fields.items()}
    values, next_history = evaluate(metrics, baseline["limits"], baseline_values,
                                    baseline["regression_tolerances"], history,
                                    not reasons, noisy)
    for name, item in values.items():
        item["delta_from_baseline"] = None if item["value"] is None else item["value"] - baseline_values[name]
    status = "incomparable" if reasons else (
        "fail" if any(x["status"] == "fail" for x in values.values()) else
        "warning" if any(x["status"] == "warning" for x in values.values()) else
        "noisy" if any(x["status"] == "noisy" for x in values.values()) else "pass")
    return values, next_history, not reasons, reasons, status


def build_dependencies(binary: Path) -> list[dict[str, str]]:
    output = run_checked(["otool", "-L", str(binary)])
    rows = []
    for line in output.splitlines()[1:]:
        identity = line.strip()
        if identity:
            rows.append({"install_name": identity.split(" (compatibility version", 1)[0], "identity": identity})
    return rows


def make_result(commit: str, dirty: bool, source_hash: str, raw: dict[str, Any], baseline: dict[str, Any], cases: dict[str, Any], gate_values: dict[str, Any], comparable: bool, reasons: list[str], gate_status: str) -> dict[str, Any]:
    binary = ROOT / "build/mac-release/bin/svgworkbench"
    bench_binary = ROOT / "build/mac-release/bin/svg_index_bench"
    size = binary.stat().st_size
    limits = baseline["limits"]
    return {
        "schema_version": 1,
        "run_timestamp_utc": datetime.now(timezone.utc).isoformat(timespec="seconds").replace("+00:00", "Z"),
        "build_sha": commit,
        "build": {"configuration": "Release", "executable": str(binary.relative_to(ROOT)), "executable_sha256": sha256(binary), "executable_bytes": size, "benchmark_executable": str(bench_binary.relative_to(ROOT)), "benchmark_executable_sha256": sha256(bench_binary), "benchmark_executable_bytes": bench_binary.stat().st_size, "dependencies": build_dependencies(binary), "source_commit": commit, "working_tree_dirty": dirty, "source_diff_sha256": source_hash},
        "hardware_toolchain": {key: raw[key] for key in ("machine", "machine_model", "chip", "memory_bytes", "architecture", "os", "compiler")},
        "seed": 20261009,
        "trial_protocol": {"cold_order_trials": 5, "warm_order_trials": 10, "cache_caveat": raw["cache_caveat"]},
        "cases": cases,
        "baseline_provenance": {"work_order": baseline["source_work_order"], "commit": baseline["source_commit"], "report": baseline["source_report"], "report_sha256": baseline["source_report_sha256"]},
        "comparison": {"status": "comparable" if comparable else "incomparable", "reasons": reasons, "baseline_values": {case: baseline["cases"][case] for case in ("10MiB", "100MiB")}},
        "gate": {"status": gate_status, "metrics": gate_values, "absolute_limits": limits, "regression_tolerances": baseline["regression_tolerances"]},
        "stretch_targets": {"targets": baseline["stretch_targets"], "executable_bytes": size, "executable_status": "report_only", "executable_reason": "measured Release output is not separately stripped; 10 MiB stripped-binary target is aspirational", "app_bundle": {"bytes": None, "status": "unavailable", "reason": "no macOS app-bundle packaging target exists in this work order"}, "pan_zoom_event_to_present_p95_ns": {"value": None, "status": "unavailable", "reason": "navigation test validates event behavior but does not time presentation"}},
    }


def collect(output: Path, history_path: Path, baseline_path: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise GateError("perf-gate requires Apple Silicon macOS")
    baseline = validate_baseline(load_json(baseline_path, "baseline"))
    commit, dirty, source_hash = source_metadata()
    raw = run_raw_campaign()
    cases, noisy = summarize_cases(raw)
    gate_values, next_history, comparable, reasons, status = gate_report(
        baseline, cases, raw, history_path, noisy)
    payload = make_result(commit, dirty, source_hash, raw, baseline, cases,
                          gate_values, comparable, reasons, status)
    validate_result(payload)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(payload, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    history_path.parent.mkdir(parents=True, exist_ok=True)
    history_path.write_text(json.dumps(next_history, indent=2, sort_keys=True, allow_nan=False) + "\n", encoding="utf-8")
    return payload, next_history

def main() -> int:
    parser = argparse.ArgumentParser(description="Build and run the macOS Release index performance gate (5 cold-order + 10 warm-order trials per case).")
    parser.add_argument("--output", type=Path, default=Path(".cache/perf-gate/result.json"), help="result JSON path under .cache/ (default: .cache/perf-gate/result.json)")
    parser.add_argument("--history", type=Path, default=Path(".cache/perf-gate/history.json"), help="per-metric streak JSON path under .cache/")
    parser.add_argument("--baseline", type=Path, default=BASELINE_PATH, help="pinned baseline JSON")
    args = parser.parse_args()
    try:
        output = under_cache(args.output)
        history = under_cache(args.history)
        baseline = args.baseline if args.baseline.is_absolute() else ROOT / args.baseline
        payload, _ = collect(output, history, baseline)
        print(json.dumps({"status": payload["gate"]["status"], "comparison": payload["comparison"]["status"], "output": str(output.relative_to(ROOT))}, sort_keys=True))
        return 1 if payload["gate"]["status"] == "fail" else 0
    except (GateError, OSError, KeyError, TypeError) as exc:
        print(f"perf-gate: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
