#!/usr/bin/env python3
"""Run bounded sanitizer fuzz campaigns, worker TSan, and a render RSS soak."""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_OUTPUT = ROOT / ".cache/soak/latest.json"
DEFAULT_MINUTES = 5
MAX_MINUTES = 20
SEED = 20261010
MEMORY_ITERATIONS = 10000
MEMORY_GROWTH_LIMIT = 64 * 1024 * 1024
MEMORY_ABSOLUTE_LIMIT = 512 * 1024 * 1024


class SoakError(ValueError):
    """Raised when a child report or the consolidated schema is invalid."""


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--minutes", type=int, default=DEFAULT_MINUTES)
    parser.add_argument("--seed", type=int, default=SEED)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    args = parser.parse_args(argv)
    if not 1 <= args.minutes <= MAX_MINUTES:
        parser.error(f"minutes must be in 1..{MAX_MINUTES}")
    if not 0 <= args.seed <= 0xFFFFFFFF:
        parser.error("seed must be an unsigned 32-bit integer")
    resolved_output = args.output.resolve() if args.output.is_absolute() else (ROOT / args.output).resolve()
    cache_root = (ROOT / ".cache").resolve()
    if not cache_root.is_relative_to(ROOT.resolve()) or not resolved_output.is_relative_to(cache_root / "soak"):
        parser.error("output must stay below repository .cache/soak")
    args.output = resolved_output
    return args


def validate_memory_report(value: object, expected_iterations: int = MEMORY_ITERATIONS) -> None:
    keys = {"schema_version", "iterations", "completed_iterations", "baseline_rss_bytes",
            "peak_rss_bytes", "growth_bytes", "growth_limit_bytes", "absolute_limit_bytes",
            "pixel_checksum", "result"}
    if not isinstance(value, dict) or set(value) != keys:
        raise SoakError("memory report has an invalid schema")
    if value["schema_version"] != 1 or isinstance(value["schema_version"], bool):
        raise SoakError("memory report schema_version must be integer 1")
    for key in ("iterations", "completed_iterations", "baseline_rss_bytes", "peak_rss_bytes",
                "growth_bytes", "growth_limit_bytes", "absolute_limit_bytes", "pixel_checksum"):
        if isinstance(value[key], bool) or not isinstance(value[key], int) or value[key] < 0:
            raise SoakError(f"memory report {key} must be a nonnegative integer")
    if value["iterations"] != expected_iterations or value["completed_iterations"] != expected_iterations:
        raise SoakError("memory report did not complete the requested render count")
    if value["growth_limit_bytes"] != MEMORY_GROWTH_LIMIT or value["absolute_limit_bytes"] != MEMORY_ABSOLUTE_LIMIT:
        raise SoakError("memory report limits do not match the frozen soak contract")
    if value["peak_rss_bytes"] < value["baseline_rss_bytes"] or value["growth_bytes"] != value["peak_rss_bytes"] - value["baseline_rss_bytes"]:
        raise SoakError("memory report RSS measurements are inconsistent")
    if value["result"] not in ("pass", "fail"):
        raise SoakError("memory report result is invalid")
    if value["result"] == "pass" and (value["growth_bytes"] > MEMORY_GROWTH_LIMIT or
                                        value["peak_rss_bytes"] > MEMORY_ABSOLUTE_LIMIT):
        raise SoakError("memory report passed outside its RSS limits")


def validate_report(report: object) -> None:
    keys = {"schema_version", "kind", "status", "seed", "minutes", "fuzz_budget_seconds",
            "fuzz_campaign_seconds_each", "memory_iterations", "memory_growth", "phases",
            "child_reports", "elapsed_seconds", "output_path"}
    if not isinstance(report, dict) or set(report) != keys:
        raise SoakError("soak report has an invalid top-level schema")
    if report["schema_version"] != 1 or isinstance(report["schema_version"], bool):
        raise SoakError("soak report schema_version must be integer 1")
    if report["kind"] != "vexel_soak" or report["status"] not in ("pass", "failure"):
        raise SoakError("soak report kind or status is invalid")
    for key in ("seed", "minutes", "fuzz_budget_seconds", "fuzz_campaign_seconds_each", "memory_iterations"):
        if isinstance(report[key], bool) or not isinstance(report[key], int) or report[key] < 0:
            raise SoakError(f"soak report {key} must be a nonnegative integer")
    if report["seed"] > 0xFFFFFFFF or not 1 <= report["minutes"] <= MAX_MINUTES:
        raise SoakError("soak report seed or minutes are outside limits")
    if report["fuzz_budget_seconds"] != report["minutes"] * 60 or report["fuzz_campaign_seconds_each"] * 2 != report["fuzz_budget_seconds"]:
        raise SoakError("soak report fuzz budget is inconsistent")
    if report["memory_iterations"] != MEMORY_ITERATIONS:
        raise SoakError("soak report memory iteration count is invalid")
    if not isinstance(report["elapsed_seconds"], (int, float)) or isinstance(report["elapsed_seconds"], bool) or report["elapsed_seconds"] < 0:
        raise SoakError("soak report elapsed_seconds is invalid")
    if not isinstance(report["output_path"], str) or not isinstance(report["phases"], list) or not isinstance(report["child_reports"], dict):
        raise SoakError("soak report paths or phase list are invalid")
    names: set[str] = set()
    for phase in report["phases"]:
        phase_keys = {"name", "status", "command", "return_code", "elapsed_seconds", "stdout_log", "stderr_log", "timeout_seconds"}
        if not isinstance(phase, dict) or set(phase) != phase_keys:
            raise SoakError("soak phase has an invalid schema")
        if not isinstance(phase["name"], str) or phase["name"] in names:
            raise SoakError("soak phase names must be unique strings")
        names.add(phase["name"])
        if phase["status"] not in ("pass", "failure", "timed_out", "skipped"):
            raise SoakError("soak phase status is invalid")
        if not isinstance(phase["command"], list) or not all(isinstance(arg, str) for arg in phase["command"]):
            raise SoakError("soak phase command must be an argv string array")
        return_code = phase["return_code"]
        if return_code is not None and (isinstance(return_code, bool) or not isinstance(return_code, int)):
            raise SoakError("soak phase return_code must be integer or null")
        for key in ("elapsed_seconds", "timeout_seconds"):
            if not isinstance(phase[key], (int, float)) or isinstance(phase[key], bool) or phase[key] < 0:
                raise SoakError(f"soak phase {key} is invalid")
        if not all(isinstance(phase[key], str) for key in ("stdout_log", "stderr_log")):
            raise SoakError("soak phase log paths must be strings")
    required_names = (
        "release-configure", "release-build", "render-memory-growth", "asan-configure",
        "asan-fuzzer-build", "xml-fuzz", "shape-fuzz", "tsan-configure", "tsan-build",
        "tsan-worker-queue",
    )
    if [phase["name"] for phase in report["phases"]] != list(required_names):
        raise SoakError("soak report must contain every phase once and in execution order")
    if (report["status"] == "pass") != all(phase["status"] == "pass" for phase in report["phases"]):
        raise SoakError("soak report status does not match phase status")
    if report["phases"][2]["status"] == "pass" and report["memory_growth"] is None:
        raise SoakError("a passed memory phase must include its validated RSS report")
    if report["memory_growth"] is not None:
        validate_memory_report(report["memory_growth"])


class Runner:
    def __init__(self, seed: int, campaign_seconds_each: int) -> None:
        self.seed = seed
        self.campaign_seconds_each = campaign_seconds_each
        self.phases: list[dict[str, Any]] = []
        self.child_reports: dict[str, str] = {}
        self.memory_growth: dict[str, Any] | None = None
        self.log_root = ROOT / ".cache/soak/logs"
        self.log_root.mkdir(parents=True, exist_ok=True)

    def run(self, name: str, argv: list[str], timeout: float) -> dict[str, Any]:
        safe_name = name.replace("/", "-")
        stdout_path = self.log_root / f"{safe_name}.stdout.txt"
        stderr_path = self.log_root / f"{safe_name}.stderr.txt"
        started = time.monotonic()
        try:
            completed = subprocess.run(argv, cwd=ROOT, capture_output=True, text=True,
                                       timeout=timeout, check=False)
            stdout = completed.stdout
            stderr = completed.stderr
            return_code: int | None = completed.returncode
            timed_out = False
        except subprocess.TimeoutExpired as exc:
            stdout = exc.stdout or ""
            stderr = exc.stderr or ""
            if isinstance(stdout, bytes):
                stdout = stdout.decode("utf-8", "replace")
            if isinstance(stderr, bytes):
                stderr = stderr.decode("utf-8", "replace")
            return_code = None
            timed_out = True
        elapsed = time.monotonic() - started
        stdout_path.write_text(stdout, encoding="utf-8")
        stderr_path.write_text(stderr, encoding="utf-8")
        status = "timed_out" if timed_out else ("pass" if return_code == 0 else "failure")
        phase = {"name": name, "status": status, "command": argv, "return_code": return_code,
                 "elapsed_seconds": round(elapsed, 6), "stdout_log": str(stdout_path.relative_to(ROOT)),
                 "stderr_log": str(stderr_path.relative_to(ROOT)), "timeout_seconds": timeout}
        self.phases.append(phase)
        return {"phase": phase, "stdout": stdout, "stderr": stderr}

    def skip(self, name: str, argv: list[str], reason: str) -> None:
        phase = {"name": name, "status": "skipped", "command": argv, "return_code": None,
                 "elapsed_seconds": 0.0, "stdout_log": reason, "stderr_log": "",
                 "timeout_seconds": 0.0}
        self.phases.append(phase)


def _read_child_report(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        raise SoakError(f"could not read child report {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise SoakError(f"child report {path} is not a JSON object")
    return value


def validate_xml_child(report: dict[str, Any], seed: int, duration: int) -> None:
    reported_seed = report.get("seed")
    if (report.get("status") != "pass" or isinstance(reported_seed, bool) or reported_seed != seed or
            isinstance(report.get("iterations"), bool) or
            not isinstance(report.get("iterations"), int) or report["iterations"] < 1 or
            report.get("timed_out") is not False or report.get("sanitizer_output") is not False or
            report.get("duration_seconds") != duration):
        raise SoakError("XML campaign report did not prove a clean seeded run")


def validate_shape_child(report: dict[str, Any], seed: int, duration: int) -> None:
    cases = report.get("cases")
    cpu_seconds = report.get("process_cpu_seconds")
    report_seed = report.get("seed")
    zero_fields = ("sanitizer_reports", "crashes", "invariant_failures", "sanitizer_stderr_bytes")
    if (report.get("result") != "pass" or report.get("seed") != seed or
            isinstance(report_seed, bool) or
            isinstance(cases, bool) or not isinstance(cases, int) or cases < 1 or
            isinstance(cpu_seconds, bool) or not isinstance(cpu_seconds, (int, float)) or
            cpu_seconds < 0 or any(isinstance(report.get(key), bool) or
                                   not isinstance(report.get(key), int) or report[key] != 0
                                   for key in zero_fields) or
            (cases < 1_000_000 and cpu_seconds < duration)):
        raise SoakError("shape campaign report did not prove a clean bounded seeded run")


def run_soak(args: argparse.Namespace) -> dict[str, Any]:
    started = time.monotonic()
    campaign_each = args.minutes * 30
    runner = Runner(args.seed, campaign_each)
    py = sys.executable
    release_ok = True
    for name, command in (
        ("release-configure", ["cmake", "--preset", "mac-release"]),
        ("release-build", ["cmake", "--build", "--preset", "mac-release", "--target", "render_soak_driver"]),
    ):
        result = runner.run(name, command, 900.0)
        if result["phase"]["status"] != "pass":
            release_ok = False

    memory_command = [str(ROOT / "build/mac-release/bin/render_soak_driver"), "--input",
                      str(ROOT / "tests/svg/shape/fixtures/rect.svg"), "--iterations", str(MEMORY_ITERATIONS)]
    if release_ok:
        memory = runner.run("render-memory-growth", memory_command, 600.0)
        if memory["phase"]["status"] == "pass":
            try:
                report = json.loads(memory["stdout"].strip())
                validate_memory_report(report)
                if report["result"] != "pass":
                    raise SoakError("render memory driver exceeded an RSS bound")
                runner.memory_growth = report
            except (json.JSONDecodeError, SoakError) as exc:
                memory["phase"]["status"] = "failure"
                memory["phase"]["stderr_log"] = f"invalid memory report: {exc}"
    else:
        runner.skip("render-memory-growth", memory_command, "release driver build failed")

    asan_configure = runner.run("asan-configure", ["cmake", "--preset", "mac-asan"], 900.0)
    asan_build_command = ["cmake", "--build", "--preset", "mac-asan", "--target",
                          "xml_tokenizer_fuzz", "shape_fuzz"]
    asan_build = runner.run("asan-fuzzer-build", asan_build_command, 900.0) if asan_configure["phase"]["status"] == "pass" else None
    if asan_build is None:
        runner.skip("asan-fuzzer-build", asan_build_command, "ASan configure failed")
    fuzz_build_ok = asan_build is not None and asan_build["phase"]["status"] == "pass"

    xml_output = ROOT / ".cache/WO-M2-03/soak-xml-latest.json"
    xml_command = [py, "scripts/xml_fuzz_campaign.py", "--binary", "build/mac-asan/bin/xml_tokenizer_fuzz",
                   "--duration", str(campaign_each), "--seed", str(args.seed), "--output",
                   str(xml_output.relative_to(ROOT))]
    shape_output = ROOT / ".cache/soak/shape-latest.json"
    shape_command = [py, "tests/fuzz/shape/shape_fuzz.py", "--binary", "build/mac-asan/bin/shape_fuzz",
                     "--duration", str(campaign_each), "--seed", str(args.seed), "--cases", "1000000",
                     "--output", str(shape_output.relative_to(ROOT))]
    if fuzz_build_ok:
        xml = runner.run("xml-fuzz", xml_command, float(campaign_each + 30))
        runner.child_reports["xml_fuzz"] = str(xml_output.relative_to(ROOT))
        if xml["phase"]["status"] == "pass":
            try:
                xml_report = _read_child_report(xml_output)
                validate_xml_child(xml_report, args.seed, campaign_each)
            except (OSError, ValueError, SoakError) as exc:
                xml["phase"]["status"] = "failure"
                xml["phase"]["stderr_log"] = f"invalid XML report: {exc}"
        shape = runner.run("shape-fuzz", shape_command, float(campaign_each + 30))
        runner.child_reports["shape_fuzz"] = str(shape_output.relative_to(ROOT))
        if shape["phase"]["status"] == "pass":
            try:
                shape_report = _read_child_report(shape_output)
                validate_shape_child(shape_report, args.seed, campaign_each)
            except (OSError, ValueError, SoakError) as exc:
                shape["phase"]["status"] = "failure"
                shape["phase"]["stderr_log"] = f"invalid shape report: {exc}"
    else:
        runner.skip("xml-fuzz", xml_command, "ASan fuzzer build failed")
        runner.skip("shape-fuzz", shape_command, "ASan fuzzer build failed")

    tsan_steps = (
        ("tsan-configure", ["cmake", "--preset", "mac-tsan"]),
        ("tsan-build", ["cmake", "--build", "--preset", "mac-tsan", "--target", "job_snapshot_test"]),
        ("tsan-worker-queue", ["ctest", "--test-dir", "build/mac-tsan", "-R", "^job-snapshot$", "--output-on-failure"]),
    )
    tsan_ok = True
    for name, command in tsan_steps:
        result = runner.run(name, command, 900.0) if tsan_ok else None
        if result is None or result["phase"]["status"] != "pass":
            tsan_ok = False
            if result is None:
                runner.skip(name, command, "previous TSan step failed")

    report: dict[str, Any] = {
        "schema_version": 1,
        "kind": "vexel_soak",
        "status": "pass" if all(phase["status"] == "pass" for phase in runner.phases) else "failure",
        "seed": args.seed,
        "minutes": args.minutes,
        "fuzz_budget_seconds": args.minutes * 60,
        "fuzz_campaign_seconds_each": campaign_each,
        "memory_iterations": MEMORY_ITERATIONS,
        "memory_growth": runner.memory_growth,
        "phases": runner.phases,
        "child_reports": runner.child_reports,
        "elapsed_seconds": round(time.monotonic() - started, 6),
        "output_path": str(args.output),
    }
    validate_report(report)
    return report


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        report = run_soak(args)
        args.output.parent.mkdir(parents=True, exist_ok=True)
        args.output.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    except (OSError, SoakError, subprocess.SubprocessError) as exc:
        print(f"soak failed before report completion: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
