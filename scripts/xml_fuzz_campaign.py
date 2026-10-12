#!/usr/bin/env python3
"""Bounded, deterministic supervisor for the native XML tokenizer fuzzer."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import signal
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MAX_DURATION = 600
MAX_INPUT = 2048
CANARY_CODE = (
    "import sys; data=bytes.fromhex(sys.argv[1]); "
    "('synthetic crash canary' in data.decode('ascii', 'ignore')) and "
    "(_ for _ in ()).throw(RuntimeError('SYNTHETIC_HARNESS_CANARY'))"
)


def confined_output_path(path: str | Path, root: Path = ROOT) -> Path:
    candidate = Path(path)
    if not candidate.is_absolute():
        candidate = root / candidate
    resolved = candidate.resolve()
    base = root.resolve()
    if not any(resolved.is_relative_to(base / part) for part in (".cache", "artifacts/WO-M2-03")):
        raise ValueError("output must stay below .cache or artifacts/WO-M2-03")
    return resolved


def normalize_stack(output: bytes | str) -> str:
    text = output.decode("utf-8", "replace") if isinstance(output, bytes) else output
    frames: list[str] = []
    for line in text.splitlines():
        if line.lstrip().startswith("File "):
            frame = re.sub(r'File "[^"]+"', 'File "<path>"', line.strip())
            frame = re.sub(r", line \d+", ", line <n>", frame)
            frames.append(frame)
        elif " in " in line and ("#" in line or "0x" in line):
            frame = re.sub(r"0x[0-9a-fA-F]+", "<addr>", line.strip())
            frame = re.sub(r"/(?:private/)?var/folders/[^/\s]+/[^/\s]+/T/[^\s:)]*", "<tmp>", frame)
            frame = re.sub(r"(?<![A-Za-z0-9_])/(?:private/)?tmp/[^\s:)]*", "<tmp>", frame)
            frame = re.sub(r"/[^\s:)]*\.worktrees/[^\s:)]*", "<worktree>", frame)
            frame = re.sub(r"(<tmp>):\d+", r"\1:<n>", frame)
            frames.append(frame)
        elif re.search(r"(?:AddressSanitizer|UndefinedBehaviorSanitizer|RuntimeError|SYNTHETIC_HARNESS_CANARY|invariant failed|ERROR:)", line):
            frames.append(line.strip())
    return "\n".join(frames) or "process failure without stack text"


def fingerprint(output: bytes | str) -> str:
    return hashlib.sha256(normalize_stack(output).encode("utf-8")).hexdigest()


def kill_process_group(process: subprocess.Popen[bytes], grace: float = 0.15) -> None:
    try:
        os.killpg(process.pid, signal.SIGTERM)
    except ProcessLookupError:
        pass
    try:
        process.wait(timeout=grace)
    except subprocess.TimeoutExpired:
        pass
    try:
        os.killpg(process.pid, signal.SIGKILL)
    except ProcessLookupError:
        pass
    process.wait()


def supervised(argv: list[str], timeout: float, input_data: bytes | None = None) -> dict[str, object]:
    process = subprocess.Popen(argv, stdin=subprocess.PIPE if input_data is not None else subprocess.DEVNULL,
                               stdout=subprocess.PIPE, stderr=subprocess.PIPE, start_new_session=True)
    timed_out = False
    try:
        stdout, stderr = process.communicate(input=input_data, timeout=timeout)
    except subprocess.TimeoutExpired:
        timed_out = True
        kill_process_group(process)
        stdout, stderr = process.communicate()
    group_alive = True
    try:
        os.killpg(process.pid, 0)
    except ProcessLookupError:
        group_alive = False
    return {"argv": argv, "returncode": process.returncode, "timed_out": timed_out,
            "stdout": stdout, "stderr": stderr, "group_terminated": not group_alive}


def canary_run(data: bytes) -> dict[str, object]:
    return supervised([sys.executable, "-c", CANARY_CODE, data.hex()], 3.0)


def minimize(data: bytes, reproduces, budget: int = 96, max_seconds: float = 15.0) -> bytes:
    current = data
    attempts = 0
    granularity = 2
    deadline = time.monotonic() + max_seconds
    while current and attempts < budget and time.monotonic() < deadline:
        chunk = max(1, len(current) // granularity)
        changed = False
        start = 0
        while start < len(current) and attempts < budget and time.monotonic() < deadline:
            candidate = current[:start] + current[min(len(current), start + chunk):]
            attempts += 1
            if len(candidate) < len(current) and reproduces(candidate):
                current = candidate
                granularity = max(2, granularity - 1)
                changed = True
                break
            start += chunk
        if not changed:
            if granularity >= len(current):
                break
            granularity = min(len(current), granularity * 2)
    return current


def validate_regression_records(records: object, seed_root: Path) -> dict[str, Path]:
    if not isinstance(records, list):
        raise ValueError("regressions must be an array")
    mapping: dict[str, Path] = {}
    for record in records:
        if not isinstance(record, dict):
            raise ValueError("regression record must be an object")
        issue_id = record.get("issue_id")
        if not isinstance(issue_id, str) or not re.fullmatch(r"XML-FUZZ-\d{4}", issue_id) or issue_id in mapping:
            raise ValueError("regression issue IDs must be unique XML-FUZZ-NNNN values")
        seed_name = record.get("seed_file")
        digest = record.get("input_sha256")
        if not isinstance(seed_name, str) or not isinstance(digest, str) or not re.fullmatch(r"[0-9a-f]{64}", digest):
            raise ValueError("regression must name a seed file and SHA-256")
        fingerprint_value = record.get("fingerprint")
        if not isinstance(fingerprint_value, str) or not re.fullmatch(r"[0-9a-f]{64}", fingerprint_value):
            raise ValueError("regression must record a normalized stack fingerprint")
        path = (seed_root / seed_name).resolve()
        if not path.is_relative_to(seed_root.resolve()) or not path.is_file():
            raise ValueError("regression seed escapes seed root or is missing")
        if hashlib.sha256(path.read_bytes()).hexdigest() != digest:
            raise ValueError("regression seed hash does not match recorded input hash")
        if record.get("replay_test") != "tests.test_xml_fuzz_campaign.TestRegressionMap":
            raise ValueError("regression does not identify the deterministic replay test")
        mapping[issue_id] = path
    return mapping


def write_json(path: Path, report: dict[str, object]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(report, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def run_canary(output: Path) -> int:
    original = (ROOT / "tests/svg/xml/fuzz/seeds/synthetic-canary.seed").read_bytes()
    initial = canary_run(original)
    raw = initial["stderr"]
    expected = fingerprint(raw)
    minimized = minimize(original, lambda item: initial_reproduces(item, expected), budget=64)
    replay = canary_run(minimized)
    same = fingerprint(replay["stderr"]) == expected
    failure_dir = confined_output_path("artifacts/WO-M2-03/failures/synthetic-canary")
    failure_dir.mkdir(parents=True, exist_ok=True)
    (failure_dir / "original.input").write_bytes(original)
    (failure_dir / "minimized.input").write_bytes(minimized)
    (failure_dir / "raw-output.txt").write_bytes(raw)
    report = {"kind": "synthetic_harness_canary", "synthetic": True, "seed": 0,
              "iteration": 0, "original_sha256": hashlib.sha256(original).hexdigest(),
              "minimized_sha256": hashlib.sha256(minimized).hexdigest(), "original_size": len(original),
              "minimized_size": len(minimized), "fingerprint": expected,
              "minimized_fingerprint": fingerprint(replay["stderr"]), "same_fingerprint": same,
              "signal_or_status": replay["returncode"], "reproduction_command": "python3 scripts/xml_fuzz_campaign.py --self-test-crash",
              "raw_output": raw.decode("utf-8", "replace"), "failure_files": str(failure_dir.relative_to(ROOT))}
    write_json(output, report)
    return 0 if same and len(minimized) <= len(original) else 1


def initial_reproduces(data: bytes, expected: str) -> bool:
    result = canary_run(data)
    return result["returncode"] != 0 and fingerprint(result["stderr"]) == expected


def run_timeout_canary(output: Path) -> int:
    child = "import subprocess,sys,time; p=subprocess.Popen([sys.executable,'-c','import time;time.sleep(30)']); print(p.pid,flush=True); time.sleep(30)"
    result = supervised([sys.executable, "-c", child], 0.2)
    report = {"kind": "synthetic_timeout_canary", "synthetic": True, "timed_out": result["timed_out"],
              "process_group_terminated": result["group_terminated"], "returncode": result["returncode"],
              "duration_limit_seconds": 0.2}
    write_json(output, report)
    return 0 if result["timed_out"] and result["group_terminated"] else 1


def replay_input(binary: Path, seed: int, iteration: int) -> bytes:
    result = subprocess.run([str(binary), "--replay", str(seed), str(iteration)], capture_output=True, timeout=3)
    try:
        return bytes.fromhex(json.loads(result.stdout.splitlines()[0])["input_hex"])
    except (ValueError, KeyError, IndexError, json.JSONDecodeError) as exc:
        raise RuntimeError("native deterministic replay did not emit bounded input") from exc


def seed_input(seed: int, iteration: int) -> bytes:
    mask = 0xFFFFFFFF

    def next_random(state: int) -> int:
        state ^= (state << 13) & mask
        state ^= state >> 17
        state ^= (state << 5) & mask
        return state & mask

    state = (seed ^ (iteration & mask) ^ ((iteration >> 32) & mask) ^ 0x9E3779B9) & mask
    if state == 0:
        state = 0x6D2B79F5
    state = next_random(state)
    length = state % MAX_INPUT
    data = bytearray()
    for _ in range(length):
        state = next_random(state)
        data.append(state & 0xFF)
    if iteration % 3 == 0:
        motifs = (b"<a/>", b"<svg xmlns='urn:x'><g></svg>", b"<!--x-->", b"<![CDATA[x]]>",
                  b"<?p x?>", b"<a x='&amp;'/>", b"<!DOCTYPE a><a/>", b"<a>]]></a>",
                  b'<a x="x" x=\'y\'/>', b"</a>")
        state = next_random(state)
        data = bytearray(motifs[state % len(motifs)])
    return bytes(data)


def one_input_command(binary: Path, data: bytes, iteration: int) -> list[str]:
    return [str(binary), "--one-input-hex", data.hex(), str(iteration)]


def input_reproduces(binary: Path, data: bytes, expected: str, iteration: int) -> bool:
    if len(data) > MAX_INPUT:
        return False
    result = supervised(one_input_command(binary, data, iteration), 2.0)
    output = result["stderr"]
    return bool(result["returncode"] != 0 and fingerprint(output) == expected)


def run_campaign(args: argparse.Namespace, output: Path) -> int:
    binary = (ROOT / args.binary).resolve()
    if not binary.is_relative_to(ROOT) or not binary.is_file():
        raise ValueError("fuzzer binary must exist inside the repository")
    command = [str(binary), "--campaign", str(args.seed), str(args.duration)]
    started = time.monotonic()
    result = supervised(command, args.duration + 5.0)
    elapsed = time.monotonic() - started
    stdout = result["stdout"]
    stderr = result["stderr"]
    combined = stdout + stderr
    sanitizer = bool(re.search(rb"AddressSanitizer|UndefinedBehaviorSanitizer|runtime error:", combined))
    clean = result["returncode"] == 0 and not result["timed_out"] and not sanitizer
    progress = [int(item) for item in re.findall(rb"PROGRESS (\d+)", stdout)]
    report: dict[str, object] = {"kind": "xml_tokenizer_campaign", "synthetic": False,
        "status": "pass" if clean else "failure", "seed": args.seed, "duration_seconds": args.duration,
        "elapsed_seconds": round(elapsed, 6), "iterations": 0, "toolchain": subprocess.run(["clang", "--version"], capture_output=True, text=True).stdout.splitlines()[0],
        "build_identity": hashlib.sha256(binary.read_bytes()).hexdigest(), "command": command,
        "timed_out": result["timed_out"], "sanitizer_output": sanitizer}
    summaries = re.findall(rb"\{[^\n]*\"iterations\"[^\n]*\}", stdout)
    if summaries:
        try:
            report["iterations"] = json.loads(summaries[-1])["iterations"]
        except (ValueError, KeyError):
            pass
    if not clean:
        report["raw_output"] = combined.decode("utf-8", "replace")
        report["signal_or_status"] = result["returncode"]
        expected = fingerprint(stderr)
        report["fingerprint"] = expected
        failures = confined_output_path("artifacts/WO-M2-03/failures")
        failures.mkdir(parents=True, exist_ok=True)
        (failures / f"seed-{args.seed}-raw-output.txt").write_bytes(combined)
        failure_iteration = None
        lower = max(0, progress[-1] if progress else 0)
        upper = lower + 128
        for iteration in range(lower, upper):
            candidate = replay_input(binary, args.seed, iteration)
            if input_reproduces(binary, candidate, expected, iteration):
                failure_iteration = iteration
                break
        if failure_iteration is not None:
            original = replay_input(binary, args.seed, failure_iteration)
            minimized = minimize(original,
                                 lambda data: input_reproduces(binary, data, expected, failure_iteration), 96)
            minimized_replay = supervised(one_input_command(binary, minimized, failure_iteration), 2.0)
            minimized_fingerprint = fingerprint(minimized_replay["stderr"])
            confirmed = minimized_replay["returncode"] != 0 and minimized_fingerprint == expected
            report.update({"iteration": failure_iteration, "fingerprint": expected,
                "original_sha256": hashlib.sha256(original).hexdigest(),
                "minimized_sha256": hashlib.sha256(minimized).hexdigest(),
                "minimized_fingerprint": minimized_fingerprint,
                "minimization_confirmed": confirmed,
                "reproduction_command": f"{binary} --replay {args.seed} {failure_iteration}"})
            (failures / f"seed-{args.seed}-iteration-{failure_iteration}.original.input").write_bytes(original)
            (failures / f"seed-{args.seed}-iteration-{failure_iteration}.minimized.input").write_bytes(minimized)
            (failures / f"seed-{args.seed}-iteration-{failure_iteration}.raw-output.txt").write_bytes(stderr)
        write_json(output, report)
        return 1
    write_json(output, report)
    return 0


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--duration", type=int, default=30)
    parser.add_argument("--seed", type=int, default=20261009)
    parser.add_argument("--binary", default="build/mac-asan/bin/xml_tokenizer_fuzz")
    parser.add_argument("--output", default=".cache/WO-M2-03/latest.json")
    parser.add_argument("--self-test-crash", action="store_true")
    parser.add_argument("--self-test-timeout", action="store_true")
    args = parser.parse_args(argv)
    if not 1 <= args.duration <= MAX_DURATION or not 0 <= args.seed <= 0xFFFFFFFF:
        parser.error("duration must be 1..600 and seed must be uint32")
    try:
        args.output_path = confined_output_path(args.output)
    except ValueError as exc:
        parser.error(str(exc))
    return args


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        if args.self_test_crash:
            return run_canary(args.output_path)
        if args.self_test_timeout:
            return run_timeout_canary(args.output_path)
        return run_campaign(args, args.output_path)
    except (OSError, RuntimeError, subprocess.SubprocessError, ValueError) as exc:
        print(f"XML fuzz campaign error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
