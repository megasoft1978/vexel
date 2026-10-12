"""Focused tests for the corpus full-render diagnostics tool."""

from __future__ import annotations

import importlib.util
import json
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MODULE_PATH = ROOT / "scripts" / "corpus_render_report.py"


def _load_module():
    spec = importlib.util.spec_from_file_location("corpus_render_report", MODULE_PATH)
    module = importlib.util.module_from_spec(spec)
    assert spec.loader is not None
    sys.modules[spec.name] = module
    spec.loader.exec_module(module)
    return module


report_tool = _load_module()


class DiagnosticParsingTests(unittest.TestCase):
    def test_category_and_offset_are_parsed(self) -> None:
        stderr = (
            "some noise\n"
            "render-frame: /tmp/a.svg: unsupported at byte 915\n"
        )
        self.assertEqual(report_tool.parse_diagnostic(stderr), ("unsupported", 915))

    def test_last_diagnostic_wins(self) -> None:
        stderr = "x: limit at byte 1\ny: path at byte 22\n"
        self.assertEqual(report_tool.parse_diagnostic(stderr), ("path", 22))

    def test_missing_offset_is_not_invented(self) -> None:
        self.assertEqual(report_tool.parse_diagnostic("render-frame: boom\n"), (None, None))
        self.assertEqual(report_tool.parse_diagnostic(""), (None, None))

    def test_feature_specific_unsupported_maps_to_unsupported(self) -> None:
        self.assertEqual(
            report_tool.verdict_for("unsupported-fe-turbulence", 1, False), "unsupported"
        )

    def test_unrecognised_category_is_reported_as_error(self) -> None:
        self.assertEqual(report_tool.verdict_for("mystery", 1, False), report_tool.ERROR)

    def test_success_and_timeout_verdicts(self) -> None:
        self.assertEqual(report_tool.verdict_for(None, 0, False), report_tool.SUCCESS)
        self.assertEqual(report_tool.verdict_for(None, 124, True), report_tool.TIMEOUT)
        self.assertEqual(report_tool.verdict_for(None, 1, False), report_tool.ERROR)

    def test_excerpt_is_bounded_and_readable(self) -> None:
        payload = b"<svg>" + bytes(range(256)) + b"</svg>"
        text = report_tool.excerpt(payload, 8)
        self.assertIsNotNone(text)
        assert text is not None
        self.assertLessEqual(len(text), report_tool.EXCERPT_LIMIT + 6)
        self.assertNotIn("\x00", text)

    def test_size_parser_rejects_bad_input(self) -> None:
        self.assertEqual(report_tool.parse_size("512x256"), (512, 256))
        for bad in ("512", "0x10", "512x", "axb", "-4x4"):
            with self.assertRaises(report_tool.ReportError):
                report_tool.parse_size(bad)


class ManifestGuardTests(unittest.TestCase):
    def test_pinned_digest_is_accepted(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp)
            data = json.dumps([{"path": "a.svg", "bytes": 1}]).encode()
            (corpus / "manifest.json").write_bytes(data)
            digest = report_tool.verify_manifest_digest(
                data, __import__("hashlib").sha256(data).hexdigest()
            )
            entries, got = report_tool.load_manifest(
                corpus, __import__("hashlib").sha256(data).hexdigest()
            )
            self.assertEqual(got, digest)
            self.assertEqual(len(entries), 1)

    def test_digest_mismatch_aborts(self) -> None:
        data = b"[]"
        with self.assertRaises(report_tool.ReportError):
            report_tool.verify_manifest_digest(data)

    def test_missing_manifest_aborts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(report_tool.ReportError):
                report_tool.load_manifest(Path(tmp))

    def test_manifest_records_need_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp)
            data = json.dumps([{"bytes": 4}]).encode()
            (corpus / "manifest.json").write_bytes(data)
            import hashlib

            with self.assertRaises(report_tool.ReportError):
                report_tool.load_manifest(corpus, hashlib.sha256(data).hexdigest())


class _FakeRunner:
    """Deterministic stand-in for the render binary."""

    def __init__(self, table: dict[str, tuple[int, str, bool]]):
        self.table = table
        self.calls: list[list[str]] = []

    def __call__(self, argv, timeout):  # noqa: ANN001 - matches the runner protocol
        self.calls.append(list(argv))
        return self.table[Path(argv[2]).name]


class ReportShapeTests(unittest.TestCase):
    def _corpus(self, tmp: str) -> Path:
        corpus = Path(tmp)
        (corpus / "files").mkdir(parents=True, exist_ok=True)
        for name in ("ok.svg", "bad.svg", "big.svg"):
            (corpus / "files" / name).write_bytes(b"<svg><path d='M0 0L1 1'/></svg>")
        return corpus

    def _entries(self) -> list[dict]:
        return [
            {"id": "a", "path": "files/ok.svg", "bytes": 32, "tier": "T1", "kind": "real"},
            {"id": "b", "path": "files/bad.svg", "bytes": 900, "tier": "T2", "kind": "real"},
            {"id": "c", "path": "files/big.svg", "bytes": 4096, "tier": "T3", "kind": "generated"},
        ]

    def test_histogram_and_skipped_accounting_cover_every_entry(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = self._corpus(tmp)
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: bad.svg: limit at byte 0\n", False),
                }
            )
            report = report_tool.build_report(
                self._entries(), corpus, Path("fake-bin"), "64x64", 1024, 5.0,
                Path(tmp) / "scratch.ppm", runner,
            )
            self.assertEqual(report["file_count"], 3)
            self.assertEqual(sum(report["histogram"].values()), 3)
            self.assertEqual(report["histogram"]["ok"], 1)
            self.assertEqual(report["histogram"]["limit"], 1)
            self.assertEqual(report["histogram"]["skipped"], 1)
            skipped = [item for item in report["files"] if item["verdict"] == "skipped"]
            self.assertIn("--max-bytes", skipped[0]["skip_reason"])
            self.assertEqual(len(runner.calls), 2)

    def test_fake_binary_stderr_maps_to_a_verdict_and_offset(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = self._corpus(tmp)
            entry = [{"id": "b", "path": "files/bad.svg", "bytes": 32, "tier": "T1", "kind": "real"}]
            runner = _FakeRunner(
                {"bad.svg": (1, "render-frame: files/bad.svg: unsupported at byte 7\n", False)}
            )
            report = report_tool.build_report(
                entry, corpus, Path("fake-bin"), "64x64", 1024, 5.0,
                Path(tmp) / "scratch.ppm", runner,
            )
            item = report["files"][0]
            self.assertEqual(item["verdict"], "unsupported")
            self.assertEqual(item["diagnostic_offset"], 7)
            self.assertEqual(item["exit_code"], 1)
            self.assertIsNotNone(item["excerpt"])

    def test_missing_source_is_reported_not_crashed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = self._corpus(tmp)
            entry = [{"id": "x", "path": "files/gone.svg", "bytes": 10, "tier": "T1", "kind": "real"}]
            report = report_tool.build_report(
                entry, corpus, Path("fake-bin"), "64x64", 1024, 5.0,
                Path(tmp) / "scratch.ppm", _FakeRunner({}),
            )
            self.assertEqual(report["files"][0]["verdict"], report_tool.ERROR)
            self.assertIn("missing", report["files"][0]["skip_reason"])

    def test_json_output_is_byte_identical_across_runs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = self._corpus(tmp)
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: x: xml at byte 3\n", False),
                }
            )
            outputs = []
            for run in range(2):
                report = report_tool.build_report(
                    self._entries(), corpus, Path("fake-bin"), "64x64", 1024, 5.0,
                    Path(tmp) / "scratch.ppm", runner,
                )
                target = Path(tmp) / f"report-{run}.json"
                report_tool.write_json(target, report)
                outputs.append(target.read_bytes())
            self.assertEqual(outputs[0], outputs[1])
            self.assertTrue(outputs[0].endswith(b"\n"))

    def test_markdown_names_the_diagnostic_for_every_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = self._corpus(tmp)
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: x: limit at byte 0\n", False),
                }
            )
            report = report_tool.build_report(
                self._entries(), corpus, Path("fake-bin"), "64x64", 1024, 5.0,
                Path(tmp) / "scratch.ppm", runner,
            )
            markdown = report_tool.render_markdown(report)
            self.assertIn("full-document render", markdown)
            self.assertIn("`limit`", markdown)
            self.assertIn("| 0 |", markdown)
            self.assertIn("## Files that render", markdown)


if __name__ == "__main__":
    unittest.main()
