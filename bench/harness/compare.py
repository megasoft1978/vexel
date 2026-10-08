#!/usr/bin/env python3
"""Offline, black-box SVG process benchmark harness (macOS/Apple Silicon)."""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path, PurePosixPath
import platform
import re
import resource
import selectors
import signal
import shutil
import statistics
import subprocess
import sys
import time
import xml.etree.ElementTree as ET

try:
    from bench.harness.artifact_pair import install_pair, recover_pair
    from bench.harness.errors import BenchError
    from bench.harness.png_validation import fake_png, png_dimensions
except ModuleNotFoundError:
    from artifact_pair import install_pair, recover_pair
    from errors import BenchError
    from png_validation import fake_png, png_dimensions

ROOT = Path(__file__).resolve().parents[2]
CORPUS = ROOT / "bench/corpus"
MANIFEST = CORPUS / "manifest.json"
MANIFEST_SHA = CORPUS / "manifest.sha256"
FIXTURES = ("shapes-basic", "stress-many-shapes", "stress-long-path")
DIMENSION = 256
TRIALS = 5
TIMEOUT = 60.0
OUTPUT_LIMIT = 16 * 1024
PREVIEW_LIMIT = 512
PNG_LIMIT = 2 * 1024 * 1024
MAX_ADAPTERS = 8
SANDBOX_EXEC = "/usr/bin/sandbox-exec"
PLACEHOLDERS = {"input", "output", "width", "height"}
CSV_FIELDS = (
    "record_type", "trial_id", "adapter_id", "task", "fixture_id", "fixture_sha256", "condition",
    "trial_index", "status", "exit_code", "elapsed_seconds", "peak_rss_bytes",
    "output_path", "output_size", "output_sha256", "stdout_preview",
    "stdout_bytes", "stdout_truncated", "stdout_sha256", "stderr_preview",
    "stderr_bytes", "stderr_truncated", "stderr_sha256", "argv_json", "error",
    "latency_valid_count", "latency_median", "latency_p95_nearest_rank",
    "rss_valid_count", "rss_median", "rss_p95_nearest_rank",
)


def sha(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def canonical_json(data: object) -> bytes:
    return (json.dumps(data, sort_keys=True, separators=(",", ":"), ensure_ascii=False) + "\n").encode()


def load_corpus() -> tuple[dict, dict[str, dict]]:
    raw = MANIFEST.read_bytes()
    expected = MANIFEST_SHA.read_text(encoding="ascii").strip()
    if not re.fullmatch(r"[0-9a-f]{64}", expected) or sha(raw) != expected:
        raise BenchError("pinned corpus manifest hash mismatch")
    manifest = json.loads(raw)
    records = manifest.get("fixtures")
    if not isinstance(records, list) or manifest.get("fixture_count") != len(records):
        raise BenchError("invalid pinned corpus manifest")
    found: dict[str, dict] = {}
    for rec in records:
        rel = rec.get("path")
        if (not isinstance(rel, str) or "\\" in rel or PurePosixPath(rel).is_absolute()
                or ".." in PurePosixPath(rel).parts or not rel.startswith("bench/corpus/fixtures/")):
            raise BenchError("unsafe fixture path in pinned corpus")
        path = ROOT / rel
        data = path.read_bytes()
        if sha(data) != rec.get("sha256"):
            raise BenchError(f"pinned fixture hash mismatch: {rec.get('fixture_id')}")
        validate_fixture_resources(data)
        if rec.get("fixture_id") in found:
            raise BenchError("duplicate fixture id")
        found[rec["fixture_id"]] = {**rec, "path_abs": str(path.resolve())}
    if not set(FIXTURES).issubset(found):
        raise BenchError("required benchmark fixture missing from corpus")
    return {"manifest_sha256": expected, "fixture_count": len(records)}, found


def validate_command(command: object) -> list[str]:
    if not isinstance(command, list) or not command or any(not isinstance(x, str) for x in command):
        raise BenchError("adapter command must be a non-empty argv array of strings")
    for item in command:
        if re.search(r"\{([^{}]+)\}", item):
            names = re.findall(r"\{([^{}]+)\}", item)
            if any(name not in PLACEHOLDERS for name in names):
                raise BenchError("adapter command contains unknown placeholder")
        if "{" in item or "}" in item:
            scrubbed = re.sub(r"\{(?:input|output|width|height)\}", "", item)
            if "{" in scrubbed or "}" in scrubbed:
                raise BenchError("malformed adapter placeholder")
    return command


def validate_render_placeholders(command: list[str]) -> None:
    used = {name for item in command for name in re.findall(r"\{(input|output|width|height)\}", item)}
    if used != PLACEHOLDERS:
        missing = ", ".join(sorted(PLACEHOLDERS - used))
        raise BenchError(f"render argv must use all task placeholders; missing: {missing}")


def validate_fixture_resources(data: bytes) -> None:
    if b"<!DOCTYPE" in data.upper() or b"<?XML-STYLESHEET" in data.upper():
        raise BenchError("external XML declarations are not allowed in benchmark fixtures")
    try:
        root = ET.fromstring(data)
    except ET.ParseError as exc:
        raise BenchError("benchmark fixture is not well-formed XML") from exc
    for element in root.iter():
        values = list(element.attrib.values())
        if element.text:
            values.append(element.text)
        for name, value in element.attrib.items():
            if name.rsplit("}", 1)[-1].lower() in {"href", "src"}:
                target = value.strip().lower()
                if not target.startswith("#") and not target.startswith("data:image/"):
                    raise BenchError("external SVG resource references are not allowed")
        text = " ".join(values)
        if re.search(r"@import\b", text, re.IGNORECASE):
            raise BenchError("external SVG stylesheet imports are not allowed")
        for match in re.finditer(r"url\(\s*(['\"]?)(.*?)\1\s*\)", text, re.IGNORECASE):
            if not match.group(2).strip().startswith("#"):
                raise BenchError("external SVG resource references are not allowed")


def safe_cache_target(value: str, cache: Path) -> Path:
    p = Path(value)
    if not p.is_absolute():
        p = ROOT / p
    target = Path(os.path.abspath(p))
    cache_root = Path(os.path.abspath(cache))
    repo_cache = Path(os.path.abspath(ROOT / ".cache"))
    if (repo_cache.is_symlink() or not cache_root.is_relative_to(repo_cache)
            or target == cache_root or not target.is_relative_to(cache_root)
            or target.name in {"", ".", ".."}):
        raise BenchError("unsafe output path: output must stay under repo-local .cache")
    cursor = repo_cache
    for part in target.relative_to(repo_cache).parts:
        cursor /= part
        if cursor.is_symlink():
            raise BenchError("unsafe output path: symlinked cache components are forbidden")
    resolved = target.resolve()
    if not resolved.is_relative_to(cache_root.resolve()):
        raise BenchError("unsafe output path: output must stay under repo-local .cache")
    return target


def safe_result_path(value: str) -> Path:
    path = Path(value)
    if not path.is_absolute():
        path = ROOT / path
    resolved = path.resolve()
    root = ROOT.resolve()
    allowed = ((ROOT / "bench/results").resolve(), (ROOT / ".cache").resolve())
    if any(not directory.is_relative_to(root) for directory in allowed):
        raise BenchError("result directory must resolve inside the repository")
    if not any(resolved != directory and resolved.is_relative_to(directory)
               for directory in allowed):
        raise BenchError("result path must stay under bench/results or repo-local .cache")
    return resolved


def adapter_list(config: Path | None, auto: bool) -> list[dict]:
    adapters: list[dict] = []
    if config:
        raw = json.loads(config.read_text(encoding="utf-8"))
        if not isinstance(raw, dict) or set(raw) != {"schema_version", "adapters"} or raw["schema_version"] != 1 or not isinstance(raw["adapters"], list):
            raise BenchError("adapter config schema must contain schema_version=1 and adapters")
        for item in raw["adapters"]:
            if not isinstance(item, dict) or set(item) != {"id", "path", "version_argv", "render_argv", "timeout_seconds"}:
                raise BenchError("adapter record has unsupported or missing fields")
            if not all(isinstance(item[k], str) and item[k] for k in ("id", "path")):
                raise BenchError("adapter id and path must be non-empty strings")
            if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._-]{0,63}", item["id"]):
                raise BenchError("adapter id must be a safe filename component")
            validate_command(item["version_argv"])
            validate_command(item["render_argv"])
            if item["render_argv"]:
                validate_render_placeholders(item["render_argv"])
            if not isinstance(item["timeout_seconds"], (int, float)) or not 0 < item["timeout_seconds"] <= TIMEOUT:
                raise BenchError("adapter timeout must be between 0 and 60 seconds")
            adapters.append({**item, "source": "config"})
    if auto:
        for app in ("inkscape", "donner"):
            path = shutil.which(app)
            adapters.append({"id": app, "path": path or app, "version_argv": ["--version"],
                             "render_argv": [], "timeout_seconds": TIMEOUT, "source": "auto"})
    seen: set[str] = set()
    for adapter in adapters:
        if adapter["id"] in seen:
            raise BenchError(f"duplicate adapter id: {adapter['id']}")
        seen.add(adapter["id"])
    if len(adapters) > MAX_ADAPTERS:
        raise BenchError(f"adapter count exceeds limit of {MAX_ADAPTERS}")
    return adapters


def validate_render_executable(adapter: dict, version_executable: str) -> None:
    command = adapter["render_argv"]
    if not command:
        return
    if adapter.get("source") != "offline-self-test":
        validate_render_placeholders(command)
    value = command[0]
    if "{" in value or "}" in value:
        raise BenchError("render argv must begin with its version-probed executable")
    resolved = value if os.path.isabs(value) else shutil.which(value)
    if not resolved or Path(resolved).resolve() != Path(version_executable).resolve():
        raise BenchError("render argv executable differs from the version-probed adapter path")


def _drain_process(proc: subprocess.Popen, timeout: float, buffers: list[bytearray],
                   digests: list, totals: list[int], truncated: list[bool]) -> tuple[int, str | None]:
    selector = selectors.DefaultSelector()
    streams = (proc.stdout, proc.stderr)
    for index, stream in enumerate(streams):
        os.set_blocking(stream.fileno(), False)
        selector.register(stream, selectors.EVENT_READ, index)
    deadline = time.monotonic() + timeout
    cleanup_deadline = None
    error = None
    while proc.poll() is None or selector.get_map():
        now = time.monotonic()
        if proc.poll() is None and now >= deadline:
            error, cleanup_deadline = "timeout", now + 0.25
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
        if proc.poll() is not None and cleanup_deadline is None:
            cleanup_deadline = now + 0.25
        if cleanup_deadline is not None and now >= cleanup_deadline and selector.get_map():
            error = error or "output pipe remained open after child exit"
            try:
                os.killpg(proc.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            for key in list(selector.get_map().values()):
                selector.unregister(key.fileobj)
                key.fileobj.close()
            break
        wait = 0.05 if cleanup_deadline is not None else min(0.05, max(0.0, deadline - now))
        for key, _ in selector.select(wait):
            index = key.data
            try:
                block = os.read(key.fd, 4096)
            except BlockingIOError:
                continue
            if not block:
                selector.unregister(key.fileobj)
                key.fileobj.close()
                continue
            digests[index].update(block)
            totals[index] += len(block)
            room = OUTPUT_LIMIT - len(buffers[index])
            if room > 0:
                buffers[index].extend(block[:room])
            if len(block) > max(0, room):
                truncated[index] = True
    code = proc.wait()
    for stream in streams:
        if not stream.closed:
            stream.close()
    selector.close()
    return code, error


def _limit_child_file_size() -> None:
    resource.setrlimit(resource.RLIMIT_FSIZE, (PNG_LIMIT, PNG_LIMIT))
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))


def process(argv: list[str], timeout: float, write_target: Path | None = None) -> tuple[int | None, bytes, bytes, float,
                                                                                     str | None, int | None, dict]:
    start = time.monotonic()
    buffers = [bytearray(), bytearray()]
    digests = [hashlib.sha256(), hashlib.sha256()]
    totals = [0, 0]
    truncated = [False, False]
    error = None
    code = None
    try:
        profile = '(version 1)(allow default)(deny process-fork)(deny network*)(deny file-write*)'
        if write_target is not None:
            target = safe_cache_target(str(write_target), ROOT / ".cache")
            literal = str(target).replace("\\", "\\\\").replace('"', '\\"')
            profile += f'(allow file-write* (literal "{literal}"))'
        sandboxed_argv = [SANDBOX_EXEC, "-p", profile, *argv]
        proc = subprocess.Popen(sandboxed_argv, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.PIPE, shell=False, start_new_session=True,
                                preexec_fn=_limit_child_file_size)
        code, error = _drain_process(proc, timeout, buffers, digests, totals, truncated)
    except OSError as exc:
        error = f"launch failed: {exc}"
    # Per-child peak RSS is unavailable through the bounded standard-library interface.
    rss = None
    elapsed = time.monotonic() - start
    capture = {"stdout_bytes": totals[0], "stdout_truncated": truncated[0],
               "stdout_sha256": digests[0].hexdigest(), "stderr_bytes": totals[1],
               "stderr_truncated": truncated[1], "stderr_sha256": digests[1].hexdigest()}
    return code, bytes(buffers[0]), bytes(buffers[1]), elapsed, error, rss, capture


def run_one(adapter: dict, fixture: dict, cache: Path, condition: str, index: int) -> dict:
    trial_id = f"{adapter['id']}:{fixture['fixture_id']}:{condition}:{index}"
    output = cache / "outputs" / adapter["id"] / fixture["fixture_id"] / f"{condition}-{index}.png"
    output = safe_cache_target(str(output), cache)
    output.parent.mkdir(parents=True, exist_ok=True)
    if output.exists() or output.is_symlink():
        output.unlink()
    argv = []
    error = None
    try:
        values = {"input": fixture["path_abs"], "output": str(output),
                  "width": str(DIMENSION), "height": str(DIMENSION)}
        command = validate_command(adapter["render_argv"])
        if adapter.get("source") != "offline-self-test":
            validate_render_placeholders(command)
        argv = [re.sub(r"\{(input|output|width|height)\}", lambda m: values[m.group(1)], x)
                for x in command]
        code, out, err, elapsed, error, rss, capture = process(
            argv, adapter["timeout_seconds"], output)
    except BenchError as exc:
        code, out, err, elapsed, rss, error = None, b"", b"", 0.0, None, str(exc)
    status = "passed" if error is None and code == 0 else "failed"
    output_size, output_hash = None, None
    output_path = None
    try:
        if output.is_symlink():
            raise BenchError("output must be a regular file")
        if output.is_file():
            output_path = str(output.relative_to(ROOT))
            output_size = output.stat().st_size
            if output_size <= PNG_LIMIT:
                image = output.read_bytes()
                output_hash = sha(image)
            else:
                image = b""
            if status == "passed":
                if output_size > PNG_LIMIT:
                    raise BenchError("missing or oversized output PNG")
                if png_dimensions(image) != (DIMENSION, DIMENSION):
                    raise BenchError("output PNG dimensions differ from 256x256 task")
        elif status == "passed":
            raise BenchError("missing or oversized output PNG")
    except (OSError, BenchError) as exc:
        error, status = str(exc), "failed"
    return {"trial_id": trial_id, "adapter_id": adapter["id"], "fixture_id": fixture["fixture_id"],
            "fixture_sha256": fixture["sha256"], "task": "svg-render",
            "condition": condition, "trial_index": index,
            "status": status, "exit_code": code, "elapsed_seconds": round(elapsed, 9),
            "peak_rss_bytes": rss, "output_path": output_path,
            "output_size": output_size, "output_sha256": output_hash,
            "stdout_preview": out[:PREVIEW_LIMIT].decode("utf-8", "replace"),
            "stdout_bytes": capture["stdout_bytes"], "stdout_truncated": capture["stdout_truncated"],
            "stdout_sha256": capture["stdout_sha256"],
            "stderr_preview": err[:PREVIEW_LIMIT].decode("utf-8", "replace"),
            "stderr_bytes": capture["stderr_bytes"], "stderr_truncated": capture["stderr_truncated"],
            "stderr_sha256": capture["stderr_sha256"],
            "argv": argv, "error": error}


def stats(trials: list[dict], key: str) -> dict:
    values = sorted(t[key] for t in trials if t["status"] == "passed" and t[key] is not None)
    if len(values) < TRIALS:
        return {"valid_count": len(values), "median": None, "p95_nearest_rank": None}
    return {"valid_count": len(values), "median": statistics.median(values),
            "p95_nearest_rank": values[max(0, __import__("math").ceil(.95 * len(values)) - 1)]}


def csv_bytes(trials: list[dict], summaries: list[dict] | None = None) -> bytes:
    stream = io.StringIO(newline="")
    writer = csv.DictWriter(stream, fieldnames=CSV_FIELDS, lineterminator="\n")
    writer.writeheader()
    for t in trials:
        row = {k: t.get(k) for k in CSV_FIELDS}
        row["record_type"] = "trial"
        row["argv_json"] = json.dumps(t["argv"], separators=(",", ":"), ensure_ascii=False)
        writer.writerow(row)
    for s in summaries or []:
        latency, rss = s["latency_seconds"], s["peak_rss_bytes"]
        writer.writerow({"record_type": "summary", "adapter_id": s["adapter_id"], "task": s.get("task", "svg-render"),
                         "fixture_id": s["fixture_id"], "condition": s["condition"],
                         "latency_valid_count": latency["valid_count"],
                         "latency_median": latency["median"],
                         "latency_p95_nearest_rank": latency["p95_nearest_rank"],
                         "rss_valid_count": rss["valid_count"], "rss_median": rss["median"],
                         "rss_p95_nearest_rank": rss["p95_nearest_rank"]})
    return stream.getvalue().encode("utf-8")


def machine_metadata() -> dict:
    model = "unavailable"
    memory = None
    if sys.platform == "darwin":
        try:
            code, out, _, _, _, _, _ = process(["/usr/sbin/sysctl", "-n", "hw.model"], 2.0)
            if code == 0:
                model = out.decode("ascii", "replace").strip() or "unavailable"
        except OSError:
            pass
        try:
            code, out, _, _, _, _, _ = process(["/usr/sbin/sysctl", "-n", "hw.memsize"], 2.0)
            if code == 0:
                memory = int(out.decode("ascii").strip())
        except (OSError, ValueError):
            pass
    return {"os": platform.platform(), "os_version": platform.mac_ver()[0],
            "architecture": platform.machine(), "hardware_model": model,
            "processor": platform.processor() or "unavailable", "memory_bytes": memory,
            "python": platform.python_version(),
            "cpu_count": os.cpu_count()}


def install_report(report: dict, json_path: Path, csv_path: Path, trials: list[dict], summaries: list[dict] | None = None) -> None:
    csv_data = csv_bytes(trials, summaries)
    report["csv_sha256"] = sha(csv_data)
    report["report_sha256"] = ""
    report["report_sha256"] = sha(canonical_json(report))
    install_pair(json_path, csv_path, canonical_json(report), csv_data, verify)


def verify(json_path: Path, csv_path: Path) -> None:
    raw = json_path.read_bytes()
    report = json.loads(raw)
    if raw != canonical_json(report):
        raise BenchError("JSON report encoding is not canonical")
    claimed = report.get("report_sha256")
    check = dict(report)
    check["report_sha256"] = ""
    if not claimed or sha(canonical_json(check)) != claimed:
        raise BenchError("JSON report integrity check failed")
    csv_raw = csv_path.read_bytes()
    if sha(csv_raw) != report.get("csv_sha256") or csv_raw != csv_bytes(report.get("trials", []), report.get("summary", [])):
        raise BenchError("CSV artifact hash mismatch")
    _, records = load_corpus()
    for record in report.get("fixtures", []):
        current = records.get(record.get("fixture_id"))
        if (not current or current["sha256"] != record.get("fixture_sha256")
                or current.get("source_sha256") != record.get("source_sha256")):
            raise BenchError("fixture hash mismatch against report")
    for trial in report.get("trials", []):
        if trial.get("output_sha256"):
            output = safe_cache_target(trial["output_path"], ROOT / ".cache")
            data = output.read_bytes()
            if sha(data) != trial["output_sha256"] or len(data) != trial["output_size"]:
                raise BenchError(f"output artifact tampered: {trial['trial_id']}")


def probe_adapters(adapters: list[dict]) -> list[dict]:
    availability = []
    for adapter in adapters:
        executable = adapter["path"] if os.path.isabs(adapter["path"]) else shutil.which(adapter["path"])
        if not executable or not Path(executable).exists():
            adapter["version_probe_passed"] = False
            availability.append({"id": adapter["id"], "available": False,
                                 "render_available": False,
                                 "reason": "executable not found"})
            continue
        validate_render_executable(adapter, executable)
        argv = [executable, *adapter["version_argv"]]
        code, out, err, elapsed, error, _, capture = process(
            argv, min(float(adapter["timeout_seconds"]), 10.0))
        adapter["resolved_path"] = str(Path(executable).resolve())
        adapter["version"] = (out or err).decode("utf-8", "replace").strip() if code == 0 else None
        adapter["version_sha256"] = (capture["stdout_sha256"] if capture["stdout_bytes"]
                                      else capture["stderr_sha256"]) if code == 0 else None
        adapter["version_probe"] = {"argv": argv, "exit_code": code,
            "elapsed_seconds": round(elapsed, 9), "error": error,
            **capture}
        version = adapter["version"]
        passed = code == 0 and error is None and bool(version)
        adapter["version_probe_passed"] = passed
        reason = None if passed else (error or ("version probe exited nonzero" if code != 0
                                                else "version probe returned no version text"))
        availability.append({"id": adapter["id"], "available": passed,
                              "render_available": passed and bool(adapter["render_argv"]),
                              "reason": reason if reason else (None if adapter["render_argv"]
                              else "no compatible render command declared")})
    return availability


def probe_vexel(run: bool, trials: list[dict], summary: list[dict]) -> dict:
    path = ROOT / "build/mac-debug/bin/svgworkbench"
    result = {"available": None, "argv": None, "exit_code": None,
              "elapsed_seconds": None, "version": None, "reason": None,
              "render_available": False,
              "render_unavailable_reason": "Vexel CLI has no compatible SVG render command"}
    if not run:
        result["reason"] = "smoke mode does not launch the Vexel CLI; use --run for a version probe"
    elif not path.is_file():
        result["available"] = False
        result["reason"] = "Vexel CLI build is missing"
    else:
        result["available"] = True
        argv = [str(path), "--version"]
        observations = []
        first_output = b""
        for index in range(1, TRIALS + 1):
            code, out, err, elapsed, error, rss, capture = process(argv, 10.0)
            if index == 1:
                first_output = out or err
            observations.append({"trial_id": f"vexel:cli-version:{index}", "adapter_id": "vexel",
                "task": "cli-version-probe", "fixture_id": None, "fixture_sha256": None,
                "condition": "process-cold", "trial_index": index,
                "status": "passed" if code == 0 and error is None else "failed",
                "exit_code": code, "elapsed_seconds": round(elapsed, 9), "peak_rss_bytes": rss,
                "output_path": None, "output_size": None, "output_sha256": None,
                "stdout_preview": out[:PREVIEW_LIMIT].decode("utf-8", "replace"),
                "stdout_bytes": capture["stdout_bytes"],
                "stdout_truncated": capture["stdout_truncated"],
                "stdout_sha256": capture["stdout_sha256"],
                "stderr_preview": err[:PREVIEW_LIMIT].decode("utf-8", "replace"),
                "stderr_bytes": capture["stderr_bytes"],
                "stderr_truncated": capture["stderr_truncated"],
                "stderr_sha256": capture["stderr_sha256"], "argv": argv, "error": error})
        trials.extend(observations)
        summary.append({"adapter_id": "vexel", "task": "cli-version-probe", "fixture_id": None,
                        "condition": "process-cold",
                        "latency_seconds": stats(observations, "elapsed_seconds"),
                        "peak_rss_bytes": stats(observations, "peak_rss_bytes")})
        first = observations[0]
        result.update({"argv": argv, "trial_count": len(observations),
                       "exit_code": first["exit_code"],
                       "elapsed_seconds": first["elapsed_seconds"],
                       "version": first_output.decode("utf-8", "replace").strip()
                       if first["exit_code"] == 0 else None,
                       "version_sha256": sha(first_output) if first["exit_code"] == 0 else None,
                       "reason": first["error"] or
                       ("version probe exited nonzero" if first["exit_code"] else None)})
    return result


def render_trials(adapters: list[dict], records: dict[str, dict], cache: Path,
                  enabled: bool) -> list[dict]:
    trials = []
    if not enabled:
        return trials
    for adapter in adapters:
        if not adapter.get("version_probe_passed") or "resolved_path" not in adapter:
            continue
        for fixture_id in FIXTURES:
            if not adapter["render_argv"]:
                continue
            for condition in ("cold-process", "warm-repeat"):
                for index in range(1, TRIALS + 1):
                    trials.append(run_one(adapter, records[fixture_id], cache, condition, index))
    return trials


def render_summaries(adapters: list[dict], trials: list[dict]) -> list[dict]:
    summaries = []
    for adapter in adapters:
        for fixture_id in FIXTURES:
            for condition in ("cold-process", "warm-repeat"):
                group = [t for t in trials if t["adapter_id"] == adapter["id"]
                         and t["fixture_id"] == fixture_id and t["condition"] == condition]
                if group:
                    summaries.append({"adapter_id": adapter["id"], "task": "svg-render",
                        "fixture_id": fixture_id, "condition": condition,
                        "latency_seconds": stats(group, "elapsed_seconds"),
                        "peak_rss_bytes": stats(group, "peak_rss_bytes")})
    return summaries


def build_report(mode: str, metadata: dict, corpus: dict, records: dict[str, dict],
                 adapters: list[dict], availability: list[dict], probe: dict,
                 trials: list[dict], summary: list[dict]) -> dict:
    adapter_records = [{k: v for k, v in item.items()
                        if k not in {"render_argv", "version_argv"}} | {
                            "version_argv": item["version_argv"],
                            "render_argv": item["render_argv"]} for item in adapters]
    fixtures = [{"fixture_id": fid, "fixture_sha256": records[fid]["sha256"],
                 "source_sha256": records[fid]["source_sha256"]} for fid in FIXTURES]
    limitations = ["process-cold means a fresh process only; filesystem cache eviction is not claimed",
                   "peak RSS is unavailable through the bounded standard-library process interface",
                   "no render measurements are available for adapters without a compatible declared command",
                   "Vexel --version measures startup only and is not compared with SVG rendering"]
    unavailable_hardware = [key for key, unavailable in (
        ("hardware_model", metadata.get("hardware_model") in (None, "unavailable")),
        ("memory_bytes", metadata.get("memory_bytes") is None)) if unavailable]
    if unavailable_hardware:
        limitations.append("host hardware metadata unavailable for: " + ", ".join(unavailable_hardware))
    return {"schema_version": 1, "work_order": "WO-M0-03", "mode": mode,
            "metadata": metadata,
            "limits": {"trials_per_condition": TRIALS, "timeout_seconds": TIMEOUT,
                "adapter_timeout_max_seconds": TIMEOUT, "stdout_stderr_capture_bytes": OUTPUT_LIMIT,
            "preview_bytes": PREVIEW_LIMIT, "png_max_bytes": PNG_LIMIT,
                "dimensions": [DIMENSION, DIMENSION],
                "process_policy": "sandbox-exec denies network and process forks; writes limited to designated PNG; fixture references prevalidated"},
            "corpus": corpus, "fixtures": fixtures, "adapters": adapter_records,
            "availability": availability, "vexel_version_probe": probe,
            "trials": trials, "summary": summary, "limitations": limitations}


def save_outputs(report: dict, json_path: Path, csv_path: Path,
                 trials: list[dict], summary: list[dict]) -> None:
    recover_pair(json_path, csv_path, verify)
    if json_path.exists() or csv_path.exists():
        if not json_path.is_file() or not csv_path.is_file():
            raise BenchError("refusing to overwrite an incomplete result pair")
        try:
            prior = json.loads(json_path.read_text(encoding="utf-8"))
        except (OSError, json.JSONDecodeError) as exc:
            raise BenchError("refusing to overwrite an unreadable result") from exc
        if prior.get("schema_version") != 1 or prior.get("work_order") != "WO-M0-03":
            raise BenchError("refusing to overwrite a result with different schema/work-order identity")
        try:
            verify(json_path, csv_path)
        except (BenchError, OSError, ValueError, json.JSONDecodeError) as exc:
            raise BenchError("refusing to overwrite an invalid or tampered result pair") from exc
    install_report(report, json_path, csv_path, trials, summary)


def run_mode(args: argparse.Namespace) -> int:
    cache = ROOT / ".cache/m0-03"
    corpus, records = load_corpus()
    adapters = adapter_list(Path(args.config) if args.config else None,
                            not args.no_auto_discover)
    mode = "self-test" if args.self_test_adapter else "run" if args.run else "smoke"
    if args.self_test_adapter:
        adapters = [{"id": "fake-png", "path": sys.executable, "version_argv": ["--version"],
                     "render_argv": [sys.executable, str(Path(__file__).resolve()),
                                     "--fake-worker", "{output}"],
                     "timeout_seconds": 5.0, "source": "offline-self-test"}]
    availability = probe_adapters(adapters)
    trials: list[dict] = []
    summary: list[dict] = []
    probe = probe_vexel(args.run, trials, summary)
    trials.extend(render_trials(adapters, records, cache, args.run or args.self_test_adapter))
    summary.extend(render_summaries(adapters, trials))
    report = build_report(mode, machine_metadata(), corpus, records, adapters,
                          availability, probe, trials, summary)
    save_outputs(report, safe_result_path(args.json), safe_result_path(args.csv),
                 trials, summary)
    return int(args.self_test_adapter and any(t["status"] != "passed" for t in trials))


def main() -> int:
    parser = argparse.ArgumentParser()
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--smoke", action="store_true")
    mode.add_argument("--run", action="store_true")
    mode.add_argument("--verify", metavar="JSON")
    mode.add_argument("--self-test-adapter", action="store_true")
    parser.add_argument("--json", default="bench/results/WO-M0-03.json")
    parser.add_argument("--csv", default="bench/results/WO-M0-03.csv")
    parser.add_argument("--config")
    parser.add_argument("--no-auto-discover", action="store_true")
    args = parser.parse_args()
    try:
        if sys.platform != "darwin" or platform.machine() not in {"arm64", "aarch64"}:
            raise BenchError("benchmark harness supports Apple Silicon macOS only")
        if args.verify:
            verify(safe_result_path(args.verify), safe_result_path(args.csv))
            return 0
        return run_mode(args)
    except (BenchError, OSError, ValueError, json.JSONDecodeError) as exc:
        print(f"benchmark error: {exc}", file=sys.stderr)
        return 2


def fake_worker(output_arg: str) -> int:
    output = safe_cache_target(output_arg, ROOT / ".cache")
    if output.exists() or output.is_symlink():
        raise BenchError("fake worker refuses to overwrite an existing output")
    with output.open("xb") as stream:
        stream.write(fake_png())
    return 0


if __name__ == "__main__" and len(sys.argv) > 1 and sys.argv[1] == "--fake-worker":
    try:
        if len(sys.argv) != 3:
            raise BenchError("fake worker requires one output path")
        raise SystemExit(fake_worker(sys.argv[2]))
    except Exception as exc:
        print(str(exc), file=sys.stderr)
        raise SystemExit(1)
elif __name__ == "__main__":
    raise SystemExit(main())
