#!/usr/bin/env python3
"""Generate deterministic SVGs and exercise the headless application CLI."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
import time
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_BINARY = ROOT / "build/mac-release/bin/svgworkbench"
DEFAULT_OUTPUT = ROOT / ".cache/e2e/latest.json"
DEFAULT_INPUT_ROOT = ROOT / ".cache/e2e/repro"
MAX_DOCUMENTS = 2000
MAX_TIMEOUT_SECONDS = 60
MAX_STDERR_BYTES = 8192
SANITIZER_PATTERN = re.compile(rb"AddressSanitizer|UndefinedBehaviorSanitizer|ThreadSanitizer|runtime error:")


class E2EError(ValueError):
    """Raised when inputs or generated reports violate the harness contract."""


class SeededBytes:
    """Small fixed xorshift generator whose output does not depend on Python version."""

    def __init__(self, seed: int) -> None:
        self.state = seed & 0xFFFFFFFF
        if self.state == 0:
            self.state = 0x6D2B79F5

    def next(self) -> int:
        value = self.state
        value ^= (value << 13) & 0xFFFFFFFF
        value ^= value >> 17
        value ^= (value << 5) & 0xFFFFFFFF
        self.state = value & 0xFFFFFFFF
        return self.state

    def between(self, low: int, high: int) -> int:
        return low + self.next() % (high - low + 1)

    def choose(self, items: tuple[str, ...]) -> str:
        return items[self.next() % len(items)]


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, default=DEFAULT_BINARY)
    parser.add_argument("--seed", type=int, default=20261010)
    parser.add_argument("--documents", type=int, default=256)
    parser.add_argument("--timeout-seconds", type=float, default=5.0)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--input-root", type=Path, default=DEFAULT_INPUT_ROOT)
    args = parser.parse_args(argv)
    if not 0 <= args.seed <= 0xFFFFFFFF:
        parser.error("seed must be an unsigned 32-bit integer")
    if not 1 <= args.documents <= MAX_DOCUMENTS:
        parser.error(f"documents must be in 1..{MAX_DOCUMENTS}")
    if not 0.05 <= args.timeout_seconds <= MAX_TIMEOUT_SECONDS:
        parser.error(f"timeout-seconds must be in 0.05..{MAX_TIMEOUT_SECONDS}")
    if not args.binary.is_absolute():
        args.binary = (ROOT / args.binary).resolve()
    for key, label in (("output", "report path"), ("input_root", "input root")):
        path = getattr(args, key)
        resolved = path.resolve() if path.is_absolute() else (ROOT / path).resolve()
        cache_root = (ROOT / ".cache").resolve()
        permitted = cache_root / "e2e"
        if not cache_root.is_relative_to(ROOT.resolve()) or not resolved.is_relative_to(permitted):
            parser.error(f"{label} must stay below repository .cache/e2e")
        setattr(args, key, resolved)
    return args


def generate_svg(seed: int, index: int) -> bytes:
    """Return one small valid document with stable coverage across every seed."""
    rng = SeededBytes((seed ^ ((index + 1) * 0x9E3779B9)) & 0xFFFFFFFF)
    palette = ("#1769aa", "#d1495b", "#edae49", "#3b8f67", "#7057b8", "#e8792e")
    shapes = (
        lambda: f'<rect x="{rng.between(3, 20)}" y="{rng.between(4, 18)}" width="{rng.between(12, 30)}" height="{rng.between(8, 23)}" fill="{rng.choose(("url(#linear)", "url(#radial)", rng.choose(palette)))}"/>',
        lambda: f'<circle cx="{rng.between(16, 48)}" cy="{rng.between(16, 48)}" r="{rng.between(3, 12)}" fill="{rng.choose(("url(#radial)", rng.choose(palette)))}"/>',
        lambda: f'<ellipse cx="{rng.between(16, 48)}" cy="{rng.between(16, 48)}" rx="{rng.between(4, 13)}" ry="{rng.between(3, 10)}" fill="{rng.choose(palette)}"/>',
        lambda: f'<line x1="{rng.between(2, 20)}" y1="{rng.between(2, 20)}" x2="{rng.between(40, 62)}" y2="{rng.between(40, 62)}" stroke="{rng.choose(palette)}" stroke-width="{rng.between(1, 4)}"/>',
        lambda: f'<polygon points="{rng.between(5, 20)},5 58,{rng.between(12, 25)} {rng.between(35, 55)},59" fill="{rng.choose(palette)}"/>',
        lambda: f'<path d="M 5 55 L {rng.between(20, 38)} {rng.between(8, 25)} L 59 54 Z" fill="{rng.choose(("url(#linear)", "url(#radial)", rng.choose(palette)))}"/>',
    )
    count = 3 + rng.between(0, 4)
    selected = [shapes[(index + offset) % len(shapes)]() for offset in range(count)]
    translate_x = rng.between(-2, 2)
    translate_y = rng.between(-2, 2)
    scale = rng.choose(("0.75", "0.9", "1", "1.05"))
    stops = (
        f'<stop offset="0%" stop-color="{rng.choose(palette)}" stop-opacity="{rng.choose(("0.7", "1"))}"/>',
        f'<stop offset="100%" stop-color="{rng.choose(palette)}"/>',
    )
    source_color = rng.choose(palette)
    document = (
        '<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 64 64">'
        '<defs>'
        f'<linearGradient id="linear" x1="0%" y1="0%" x2="100%" y2="100%">{"".join(stops)}</linearGradient>'
        f'<radialGradient id="radial" cx="50%" cy="50%" r="50%"><stop offset="0%" stop-color="{source_color}"/><stop offset="100%" stop-color="{rng.choose(palette)}" stop-opacity="0.6"/></radialGradient>'
        '<clipPath id="clip"><rect x="4" y="4" width="56" height="56"/></clipPath>'
        f'<g id="source"><circle cx="8" cy="8" r="5" fill="{source_color}"/></g>'
        '</defs>'
        '<rect width="64" height="64" fill="#ffffff"/>'
        f'<g transform="translate({translate_x} {translate_y})" clip-path="url(#clip)">'
        f'<g transform="scale({scale})">{"".join(selected)}</g>'
        f'<use href="#source" x="{rng.between(40, 50)}" y="{rng.between(40, 50)}"/>'
        '</g></svg>'
    )
    return document.encode("ascii")


def mutation_inputs(seed: int, source: bytes) -> list[tuple[str, bytes]]:
    rng = SeededBytes(seed ^ 0xA5A5F00D)
    flip_at = rng.between(0, len(source) - 1)
    flipped = bytearray(source)
    flipped[flip_at] ^= 0x80
    half = max(1, len(source) // 2)
    huge_number = source.replace(b'width="64"', b'width="' + b"9" * 256 + b'"', 1)
    if huge_number == source:
        huge_number = b'<svg width="' + b"9" * 256 + b'" height="64"/>'
    invalid_utf8 = source.replace(b"</svg>", b"\xff\xfe</svg>", 1)
    if invalid_utf8 == source:
        invalid_utf8 = source + b"\xff\xfe"
    return [
        ("byte-flip", bytes(flipped)),
        ("truncate-half", source[:half]),
        ("truncate-tail", source[:-1]),
        ("huge-number", huge_number),
        ("invalid-utf8", invalid_utf8),
    ]


def deep_nesting_svg(depth: int = 256) -> bytes:
    if not 1 <= depth <= 1024:
        raise E2EError("deep nesting depth must be in 1..1024")
    return (b'<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64">' +
            b"<g>" * depth + b'<rect width="64" height="64" fill="#345678"/>' +
            b"</g>" * depth + b"</svg>")


def validate_ppm(data: bytes, width: int = 64, height: int = 64) -> bool:
    match = re.match(rb"\AP6\n([0-9]{1,5}) ([0-9]{1,5})\n255\n", data)
    if match is None:
        return False
    parsed_width, parsed_height = int(match.group(1)), int(match.group(2))
    if (parsed_width, parsed_height) != (width, height):
        return False
    header_length = match.end()
    expected_payload = width * height * 3
    return len(data) == header_length + expected_payload


def validate_report(report: object) -> None:
    keys = {
        "schema_version", "kind", "seed", "status", "generated_documents",
        "mutated_inputs", "render_invocations", "valid_ppm_count", "deterministic_checks",
        "clean_stderr_checks", "crashes", "timeouts", "timeout_seconds",
        "maximum_case_seconds", "elapsed_seconds", "input_root", "failures",
    }
    if not isinstance(report, dict) or set(report) != keys:
        raise E2EError("e2e report has an invalid top-level schema")
    if report["schema_version"] != 1 or isinstance(report["schema_version"], bool):
        raise E2EError("e2e report schema_version must be integer 1")
    if report["kind"] != "svg_render_e2e":
        raise E2EError("e2e report kind is invalid")
    if isinstance(report["seed"], bool) or not isinstance(report["seed"], int) or not 0 <= report["seed"] <= 0xFFFFFFFF:
        raise E2EError("e2e report seed is invalid")
    for name in ("generated_documents", "mutated_inputs", "render_invocations", "valid_ppm_count",
                 "deterministic_checks", "clean_stderr_checks", "crashes", "timeouts"):
        if isinstance(report[name], bool) or not isinstance(report[name], int) or report[name] < 0:
            raise E2EError(f"e2e report {name} must be a nonnegative integer")
    for name in ("timeout_seconds", "maximum_case_seconds", "elapsed_seconds"):
        if isinstance(report[name], bool) or not isinstance(report[name], (int, float)) or report[name] < 0:
            raise E2EError(f"e2e report {name} must be a nonnegative number")
    if report["status"] not in ("pass", "failure") or not isinstance(report["input_root"], str):
        raise E2EError("e2e report status or input_root is invalid")
    if not isinstance(report["failures"], list):
        raise E2EError("e2e report failures must be an array")
    for item in report["failures"]:
        if not isinstance(item, dict) or set(item) != {"seed", "input_path", "reason"}:
            raise E2EError("e2e failure item has an invalid schema")
        if isinstance(item["seed"], bool) or item["seed"] != report["seed"] or not all(isinstance(item[key], str) for key in ("input_path", "reason")):
            raise E2EError("e2e failure item must identify its seed, path, and reason")
    if (report["status"] == "pass") != (len(report["failures"]) == 0):
        raise E2EError("e2e report status does not match its failures")


def _write_json(path: Path, value: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")


def _run(binary: Path, input_path: Path, output_path: Path, timeout: float) -> tuple[dict[str, Any], float]:
    argv = [str(binary), "--render-frame", str(input_path), "--size", "64x64", "--out", str(output_path)]
    started = time.monotonic()
    try:
        completed = subprocess.run(argv, cwd=ROOT, capture_output=True, timeout=timeout, check=False)
        return ({"argv": argv, "returncode": completed.returncode, "stdout": completed.stdout,
                 "stderr": completed.stderr, "timed_out": False}, time.monotonic() - started)
    except subprocess.TimeoutExpired as exc:
        return ({"argv": argv, "returncode": None, "stdout": exc.stdout or b"",
                 "stderr": exc.stderr or b"", "timed_out": True}, time.monotonic() - started)


def run_matrix(args: argparse.Namespace) -> dict[str, Any]:
    binary = args.binary.resolve()
    root = args.input_root / f"seed-{args.seed}"
    root.mkdir(parents=True, exist_ok=True)
    failures: list[dict[str, str | int]] = []
    invocations = ppm_count = deterministic = clean_stderr = crashes = timeouts = 0
    maximum_case = 0.0
    started = time.monotonic()

    def fail(path: Path, reason: str) -> None:
        failures.append({"seed": args.seed, "input_path": str(path), "reason": reason[:1024]})

    def check_mutation(path: Path, data: bytes) -> None:
        nonlocal invocations, ppm_count, crashes, timeouts, maximum_case
        path.write_bytes(data)
        output = path.with_suffix(path.suffix + ".ppm")
        run, duration = _run(binary, path, output, args.timeout_seconds)
        invocations += 1
        maximum_case = max(maximum_case, duration)
        if run["timed_out"]:
            timeouts += 1
            fail(path, f"mutation render exceeded {args.timeout_seconds:g}s")
            return
        returncode = run["returncode"]
        if not isinstance(returncode, int) or returncode < 0:
            crashes += 1
            fail(path, f"mutation process crashed with return code {returncode}")
        if len(run["stderr"]) > MAX_STDERR_BYTES:
            fail(path, f"mutation emitted {len(run['stderr'])} stderr bytes (limit {MAX_STDERR_BYTES})")
        if SANITIZER_PATTERN.search(run["stderr"]):
            fail(path, "mutation emitted sanitizer diagnostics")
        if output.is_file():
            if validate_ppm(output.read_bytes()):
                ppm_count += 1
            else:
                fail(path, "mutation produced an invalid PPM frame")

    if not binary.is_file():
        fail(binary, "renderer binary does not exist")
    else:
        for index in range(args.documents):
            source = generate_svg(args.seed, index)
            input_path = root / f"generated-{index:04d}.svg"
            input_path.write_bytes(source)
            first_output = root / f"generated-{index:04d}.first.ppm"
            second_output = root / f"generated-{index:04d}.repeat.ppm"
            first, first_duration = _run(binary, input_path, first_output, args.timeout_seconds)
            second, second_duration = _run(binary, input_path, second_output, args.timeout_seconds)
            invocations += 2
            maximum_case = max(maximum_case, first_duration, second_duration)
            if first["timed_out"] or second["timed_out"]:
                timeouts += int(first["timed_out"]) + int(second["timed_out"])
                fail(input_path, f"valid SVG exceeded {args.timeout_seconds:g}s per render")
                continue
            if first["returncode"] != 0 or second["returncode"] != 0:
                fail(input_path, f"valid SVG returned {first['returncode']} and {second['returncode']}")
            for label, run in (("first", first), ("repeat", second)):
                if run["stderr"]:
                    fail(input_path, f"{label} render emitted stderr: {run['stderr'][:256].decode('utf-8', 'replace')}")
                else:
                    clean_stderr += 1
            if first_output.is_file() and second_output.is_file():
                first_ppm = first_output.read_bytes()
                second_ppm = second_output.read_bytes()
                if validate_ppm(first_ppm) and validate_ppm(second_ppm):
                    ppm_count += 2
                else:
                    fail(input_path, "valid SVG render produced an invalid PPM")
                if hashlib.sha256(first_ppm).digest() == hashlib.sha256(second_ppm).digest():
                    deterministic += 1
                else:
                    fail(input_path, "repeat render produced a different PPM hash")
            else:
                fail(input_path, "valid SVG render did not produce both PPM outputs")

            if index < min(args.documents, 32):
                for kind, mutated in mutation_inputs(args.seed ^ index, source):
                    mutation_path = root / f"mutated-{index:04d}-{kind}.svg"
                    check_mutation(mutation_path, mutated)

        deep_path = root / "mutated-deep-nesting.svg"
        check_mutation(deep_path, deep_nesting_svg())
        malformed_path = root / "mutated-truncated-root.svg"
        check_mutation(malformed_path, b'<svg xmlns="http://www.w3.org/2000/svg"><g><path d="M 0 0 L 5 5"')

    report: dict[str, Any] = {
        "schema_version": 1,
        "kind": "svg_render_e2e",
        "seed": args.seed,
        "status": "failure" if failures else "pass",
        "generated_documents": args.documents,
        "mutated_inputs": min(args.documents, 32) * 5 + 2,
        "render_invocations": invocations,
        "valid_ppm_count": ppm_count,
        "deterministic_checks": deterministic,
        "clean_stderr_checks": clean_stderr,
        "crashes": crashes,
        "timeouts": timeouts,
        "timeout_seconds": args.timeout_seconds,
        "maximum_case_seconds": round(maximum_case, 6),
        "elapsed_seconds": round(time.monotonic() - started, 6),
        "input_root": str(root),
        "failures": failures,
    }
    validate_report(report)
    return report


def main(argv: list[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        report = run_matrix(args)
        _write_json(args.output, report)
    except (OSError, E2EError, subprocess.SubprocessError) as exc:
        print(f"SVG e2e failed: {exc}", file=sys.stderr)
        return 1
    print(json.dumps(report, sort_keys=True))
    return 0 if report["status"] == "pass" else 1


if __name__ == "__main__":
    raise SystemExit(main())
