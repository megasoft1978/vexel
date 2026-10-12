#!/usr/bin/env python3
"""Record per-file full-render diagnostics for the pinned real SVG corpus.

The existing corpus report measures a synthetic single-element preview built by
``svg_index_make_preview_source``. This tool measures the real thing: one
full-document render per manifest file through the render CLI, recording the
error category and source offset when the render fails.

Corpus files are borrowed and read-only. Nothing is copied into the repository.
The JSON output is byte-reproducible: wall-clock timing is excluded by design,
and only the Markdown summary reports elapsed milliseconds.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path
from typing import Any, Callable, Iterable, Sequence

PINNED_MANIFEST_SHA256 = "9fbf9a2f384d70a41da72e99ac259d2c5d137ffd3c8a01ce4be4741c1c29bc30"
DEFAULT_CORPUS = Path("/Users/megasoft78/Desktop/Freelance/vexel-corpus-real")
DEFAULT_BINARY = Path("build/mac-release/bin/svgworkbench")
DEFAULT_SIZE = "128x128"
DEFAULT_MAX_BYTES = 32 * 1024 * 1024
DEFAULT_TIMEOUT = 120.0
DEFAULT_TIMEOUT_GRACE = 0.0
EXCERPT_PAD = 48
EXCERPT_LIMIT = 160

# The renderer emits both lowercase categories ("limit", "xml") and camelCase
# feature categories ("unsupported-feTurbulence"), so the character class must
# accept both.
DIAGNOSTIC_RE = re.compile(
    r"^render-frame: (?P<path>.*): (?P<category>[A-Za-z][A-Za-z0-9_-]*) at byte (?P<offset>\d+)\s*$"
)
FAILURE_LINE_RE = re.compile(r"^render-frame: (?P<path>.*): (?P<message>.+?)\s*$")
USAGE_RE = re.compile(r"^(?P<width>\d{1,5})x(?P<height>\d{1,5})$")
KNOWN_CATEGORIES = (
    "none",
    "invalid-argument",
    "xml",
    "unsupported",
    "number",
    "limit",
    "path",
    "viewport",
    "oom",
)
SUCCESS = "ok"
SKIPPED = "skipped"
TIMEOUT = "timeout"
ERROR = "error"


class ReportError(RuntimeError):
    """Raised for unusable inputs; always aborts the run."""


def parse_size(text: str) -> tuple[int, int]:
    match = USAGE_RE.match(text)
    if match is None:
        raise ReportError(f"size must be WxH with positive integers, got {text!r}")
    width = int(match.group("width"))
    height = int(match.group("height"))
    if width < 1 or height < 1:
        raise ReportError(f"size must be positive, got {text!r}")
    return width, height


def _same_file(reported: str, expected: Path | None) -> bool:
    """True when a reported path and the rendered file denote the same file.

    The CLI echoes the path it was given, so an exact comparison normally
    succeeds. The trailing-components fallback tolerates a caller that passed a
    relative path; it can never match a differently named file.
    """
    if expected is None:
        return True
    if Path(reported) == expected:
        return True
    try:
        if Path(reported).resolve() == expected.resolve():
            return True
    except OSError:
        pass
    reported_parts = Path(reported).parts
    expected_parts = expected.parts
    if not reported_parts or not expected_parts:
        return False
    if reported_parts[-1] != expected_parts[-1]:
        return False
    return reported_parts[-2:] == expected_parts[-2:] or len(reported_parts) == 1


def parse_diagnostic(
    stderr: str, expected: Path | None = None
) -> tuple[str | None, int | None]:
    """Return (category, offset) for the rendered file, or (None, None).

    A diagnostic naming a different file is ignored rather than attributed to the
    current record.
    """
    for line in reversed(stderr.splitlines()):
        match = DIAGNOSTIC_RE.match(line)
        if match is not None and _same_file(match.group("path"), expected):
            return match.group("category"), int(match.group("offset"))
    return None, None


def failure_note(stderr: str, expected: Path | None = None) -> str | None:
    """Return a bounded note for a failure that carried no located diagnostic."""
    for line in reversed(stderr.splitlines()):
        match = FAILURE_LINE_RE.match(line)
        if match is not None and _same_file(match.group("path"), expected):
            return match.group("message").strip()[:200]
    stripped = [line for line in stderr.strip().splitlines() if line.strip()]
    if not stripped:
        return None
    last = stripped[-1]
    mismatch = FAILURE_LINE_RE.match(last)
    if mismatch is not None and not _same_file(mismatch.group("path"), expected):
        return None
    return last[:200]


#: Verdict vocabulary fixed by the work order. The raw category is preserved
#: separately in ``diagnostic_category``.
REPORTED_VERDICTS = (
    "ok", "unsupported", "limit", "xml", "number", "viewport", "path", "oom",
    "timeout", "error", "skipped",
)


def verdict_for(category: str | None, exit_code: int, timed_out: bool) -> str:
    if timed_out:
        return TIMEOUT
    if exit_code == 0:
        return SUCCESS
    if category is None:
        return ERROR
    if category.lower().startswith("unsupported"):
        return "unsupported"
    return category if category in REPORTED_VERDICTS else ERROR


def excerpt(data: bytes, offset: int | None) -> str | None:
    if offset is None or offset < 0 or offset >= len(data):
        return None
    start = max(0, offset - EXCERPT_PAD)
    end = min(len(data), offset + EXCERPT_PAD)
    chunk = data[start:end]
    text = "".join(chr(byte) if 32 <= byte < 127 else "." for byte in chunk)
    if len(text) > EXCERPT_LIMIT:
        text = text[:EXCERPT_LIMIT]
    return f"{'...' if start else ''}{text}{'...' if end < len(data) else ''}"


def verify_manifest_digest(data: bytes, expected: str = PINNED_MANIFEST_SHA256) -> str:
    digest = hashlib.sha256(data).hexdigest()
    if digest != expected:
        raise ReportError(
            "corpus manifest digest mismatch: "
            f"expected {expected}, found {digest}. Refusing to report on different inputs."
        )
    return digest


def load_manifest(corpus: Path, expected: str = PINNED_MANIFEST_SHA256) -> tuple[list[dict], str]:
    manifest_path = corpus / "manifest.json"
    try:
        data = manifest_path.read_bytes()
    except OSError as exc:
        raise ReportError(f"cannot read {manifest_path}: {exc}") from exc
    digest = verify_manifest_digest(data, expected)
    try:
        entries = json.loads(data)
    except json.JSONDecodeError as exc:
        raise ReportError(f"malformed manifest: {exc}") from exc
    if not isinstance(entries, list) or not entries:
        raise ReportError("manifest must be a non-empty list of file records")
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get("path"), str):
            raise ReportError("every manifest record needs a string 'path'")
    return entries, digest


def _default_runner(argv: Sequence[str], timeout: float) -> tuple[int, str, bool]:
    try:
        completed = subprocess.run(argv, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired:
        return 124, "", True
    except OSError as exc:
        return 127, str(exc), False
    return completed.returncode, completed.stderr, False


def run_trial(
    binary: Path,
    source: Path,
    size: str,
    timeout: float,
    scratch: Path,
    runner: Callable[[Sequence[str], float], tuple[int, str, bool]] = _default_runner,
    timeout_grace: float = DEFAULT_TIMEOUT_GRACE,
) -> dict[str, Any]:
    argv = [str(binary), "--render-frame", str(source), "--size", size, "--out", str(scratch)]
    effective_timeout = timeout + timeout_grace
    exit_code, stderr, timed_out = runner(argv, effective_timeout)
    category, offset = parse_diagnostic(stderr, source)
    verdict = verdict_for(category, exit_code, timed_out)
    record: dict[str, Any] = {
        "verdict": verdict,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "diagnostic_category": category,
        "diagnostic_offset": offset,
        "diagnostic_note": None,
    }
    if verdict not in (SUCCESS, SKIPPED):
        if timed_out:
            record["diagnostic_note"] = (
                f"render exceeded the {effective_timeout:g} s budget "
                f"(--timeout {timeout:g} s plus --trial-timeout-grace {timeout_grace:g} s)"
            )
        elif category is None:
            record["diagnostic_note"] = failure_note(stderr, source) or "no diagnostic output"
        try:
            payload = source.read_bytes()
        except OSError:
            payload = b""
        record["excerpt"] = excerpt(payload, offset)
    return record


def build_report(
    entries: Iterable[dict],
    corpus: Path,
    binary: Path,
    size: str,
    max_bytes: int,
    timeout: float,
    scratch: Path,
    runner: Callable[[Sequence[str], float], tuple[int, str, bool]] = _default_runner,
    timeout_grace: float = DEFAULT_TIMEOUT_GRACE,
) -> dict[str, Any]:
    files: list[dict[str, Any]] = []
    histogram: dict[str, int] = {}
    for entry in entries:
        record: dict[str, Any] = {
            "id": entry.get("id"),
            "path": entry["path"],
            "bytes": int(entry.get("bytes", 0)),
            "tier": entry.get("tier"),
            "kind": entry.get("kind"),
        }
        if record["bytes"] > max_bytes:
            record.update(
                {
                    "verdict": SKIPPED,
                    "exit_code": None,
                    "timed_out": False,
                    "diagnostic_category": None,
                    "diagnostic_offset": None,
                    "diagnostic_note": f"not render-tested: source exceeds --max-bytes ({max_bytes})",
                }
            )
        else:
            source = corpus / entry["path"]
            if not source.is_file():
                record.update(
                    {
                        "verdict": ERROR,
                        "exit_code": None,
                        "timed_out": False,
                        "diagnostic_category": None,
                        "diagnostic_offset": None,
                        "diagnostic_note": "manifest path is missing on disk",
                    }
                )
            else:
                record.update(
                    run_trial(binary, source, size, timeout, scratch, runner, timeout_grace)
                )
        histogram[record["verdict"]] = histogram.get(record["verdict"], 0) + 1
        files.append(record)
    render_tested = sum(
        1 for item in files if item["verdict"] not in (SKIPPED,)
    )
    return {
        "schema_version": 1,
        "tool": "scripts/corpus_render_report.py",
        "manifest_sha256": PINNED_MANIFEST_SHA256,
        "binary": str(binary),
        "render_size": size,
        "max_bytes": max_bytes,
        "timeout_seconds": timeout,
        "timeout_grace_seconds": timeout_grace,
        "effective_timeout_seconds": timeout + timeout_grace,
        "file_count": len(files),
        "coverage": {
            "render_tested": render_tested,
            "not_render_tested": len(files) - render_tested,
            "reason_not_render_tested": (
                f"sources larger than --max-bytes ({max_bytes}) are recorded as "
                f"{SKIPPED} and were not rendered"
            ),
        },
        "histogram": {key: histogram[key] for key in sorted(histogram)},
        "files": files,
    }


def render_markdown(report: dict[str, Any]) -> str:
    coverage = report["coverage"]
    lines = [
        "# Real-corpus full-render diagnostics",
        "",
        f"Corpus manifest SHA-256: `{report['manifest_sha256']}`.",
        f"Render binary: `{report['binary']}` at `{report['render_size']}`.",
        f"Size threshold: {report['max_bytes']} bytes (larger files are recorded as `skipped`).",
        f"Files accounted for: {report['file_count']}; "
        f"render-tested: {coverage['render_tested']}; "
        f"not render-tested: {coverage['not_render_tested']} "
        f"({coverage['reason_not_render_tested']}).",
        "",
        "Unlike `head-to-head.md`, this report measures a full-document render, not a"
        " synthetic single-element preview.",
        "",
        "## Verdict histogram",
        "",
        "| Verdict | Files |",
        "|---|---:|",
    ]
    for verdict, count in report["histogram"].items():
        lines.append(f"| `{verdict}` | {count} |")
    ok = [item for item in report["files"] if item["verdict"] == SUCCESS]
    lines += ["", "## Files that render", ""]
    if ok:
        lines += ["| ID | Bytes | Tier |", "|---|---:|---|"]
        for item in ok:
            lines.append(f"| `{item['id']}` | {item['bytes']} | {item['tier']} |")
    else:
        lines.append("None.")
    lines += ["", "## Per-file results", "", "| ID | Kind | Tier | Bytes | Verdict | Diagnostic | Offset | Excerpt |",
              "|---|---|---|---:|---|---|---:|---|"]
    for item in report["files"]:
        category = item.get("diagnostic_category") or ""
        offset = item.get("diagnostic_offset")
        excerpt_cell = (item.get("excerpt") or item.get("diagnostic_note") or "").replace("|", "\\|")
        lines.append(
            f"| `{item['id']}` | {item.get('kind') or ''} | {item.get('tier') or ''} | {item['bytes']} | "
            f"`{item['verdict']}` | {category} | {'' if offset is None else offset} | `{excerpt_cell}` |"
        )
    lines.append("")
    return "\n".join(lines)


def write_json(path: Path, payload: dict[str, Any]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    encoded = json.dumps(payload, sort_keys=True, separators=(",", ":"), ensure_ascii=False)
    path.write_text(encoded + "\n", encoding="utf-8")


def build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--corpus", default=str(DEFAULT_CORPUS))
    parser.add_argument("--binary", default=str(DEFAULT_BINARY))
    parser.add_argument("--size", default=DEFAULT_SIZE)
    parser.add_argument("--max-bytes", type=int, default=DEFAULT_MAX_BYTES)
    parser.add_argument("--timeout", type=float, default=DEFAULT_TIMEOUT)
    parser.add_argument("--trial-timeout-grace", type=float, default=DEFAULT_TIMEOUT_GRACE)
    parser.add_argument("--output-json", required=True)
    parser.add_argument("--output-markdown", required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    args = build_parser().parse_args(argv)
    try:
        parse_size(args.size)
        corpus = Path(args.corpus)
        binary = Path(args.binary)
        if args.max_bytes < 0:
            raise ReportError("--max-bytes must not be negative")
        if args.timeout <= 0:
            raise ReportError("--timeout must be positive")
        if args.trial_timeout_grace < 0:
            raise ReportError("--trial-timeout-grace must not be negative")
        if not binary.is_file():
            raise ReportError(f"render binary not found: {binary}")
        entries, _ = load_manifest(corpus)
        scratch_dir = Path(tempfile.mkdtemp(prefix="vexel-corpus-render-"))
        try:
            report = build_report(
                entries, corpus, binary, args.size, args.max_bytes, args.timeout,
                scratch_dir / "frame.ppm", timeout_grace=args.trial_timeout_grace,
            )
        finally:
            shutil.rmtree(scratch_dir, ignore_errors=True)
    except ReportError as exc:
        print(f"corpus-render-report: {exc}", file=sys.stderr)
        return 2
    write_json(Path(args.output_json), report)
    Path(args.output_markdown).parent.mkdir(parents=True, exist_ok=True)
    Path(args.output_markdown).write_text(render_markdown(report), encoding="utf-8")
    rendered = report["histogram"].get(SUCCESS, 0)
    print(
        f"corpus-render-report: {report['file_count']} files, "
        f"{rendered} rendered, histogram {report['histogram']}"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
