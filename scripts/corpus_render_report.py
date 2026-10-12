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
EXCERPT_PAD = 48
EXCERPT_LIMIT = 160

DIAGNOSTIC_RE = re.compile(r": (?P<category>[a-z][a-z0-9_-]*) at byte (?P<offset>\d+)\s*$")
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


def parse_diagnostic(stderr: str) -> tuple[str | None, int | None]:
    """Return (category, offset) from the render CLI's stderr, or (None, None)."""
    for line in reversed(stderr.splitlines()):
        match = DIAGNOSTIC_RE.search(line)
        if match is not None:
            return match.group("category"), int(match.group("offset"))
    return None, None


def verdict_for(category: str | None, exit_code: int, timed_out: bool) -> str:
    if timed_out:
        return TIMEOUT
    if exit_code == 0:
        return SUCCESS
    if category is None:
        return ERROR
    if category in KNOWN_CATEGORIES:
        return "none" if category == "none" else category
    if category.startswith("unsupported"):
        return "unsupported"
    return ERROR


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
) -> dict[str, Any]:
    argv = [str(binary), "--render-frame", str(source), "--size", size, "--out", str(scratch)]
    exit_code, stderr, timed_out = runner(argv, timeout)
    category, offset = parse_diagnostic(stderr)
    verdict = verdict_for(category, exit_code, timed_out)
    record: dict[str, Any] = {
        "verdict": verdict,
        "exit_code": exit_code,
        "timed_out": timed_out,
        "diagnostic_category": category,
        "diagnostic_offset": offset,
    }
    if verdict not in (SUCCESS, SKIPPED):
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
                    "skip_reason": f"source exceeds --max-bytes ({max_bytes})",
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
                        "skip_reason": "manifest path is missing on disk",
                    }
                )
            else:
                record.update(run_trial(binary, source, size, timeout, scratch, runner))
        histogram[record["verdict"]] = histogram.get(record["verdict"], 0) + 1
        files.append(record)
    return {
        "schema_version": 1,
        "tool": "scripts/corpus_render_report.py",
        "manifest_sha256": PINNED_MANIFEST_SHA256,
        "binary": str(binary),
        "render_size": size,
        "max_bytes": max_bytes,
        "timeout_seconds": timeout,
        "file_count": len(files),
        "histogram": {key: histogram[key] for key in sorted(histogram)},
        "files": files,
    }


def render_markdown(report: dict[str, Any]) -> str:
    lines = [
        "# Real-corpus full-render diagnostics",
        "",
        f"Corpus manifest SHA-256: `{report['manifest_sha256']}`.",
        f"Render binary: `{report['binary']}` at `{report['render_size']}`.",
        f"Size threshold: {report['max_bytes']} bytes (larger files are recorded as `skipped`).",
        f"Files accounted for: {report['file_count']}.",
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
        category = item.get("diagnostic_category") or item.get("skip_reason") or ""
        offset = item.get("diagnostic_offset")
        excerpt_cell = (item.get("excerpt") or "").replace("|", "\\|")
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
        if not binary.is_file():
            raise ReportError(f"render binary not found: {binary}")
        entries, _ = load_manifest(corpus)
        scratch_dir = Path(tempfile.mkdtemp(prefix="vexel-corpus-render-"))
        report = build_report(
            entries, corpus, binary, args.size, args.max_bytes, args.timeout,
            scratch_dir / "frame.ppm",
        )
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
