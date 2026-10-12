"""Focused tests for the corpus full-render diagnostics tool."""

from __future__ import annotations

import hashlib
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
    def test_category_and_offset_are_parsed_for_the_expected_file(self) -> None:
        stderr = "noise\nrender-frame: /tmp/a.svg: unsupported at byte 915\n"
        self.assertEqual(
            report_tool.parse_diagnostic(stderr, Path("/tmp/a.svg")), ("unsupported", 915)
        )

    def test_diagnostic_for_another_file_is_ignored(self) -> None:
        stderr = "render-frame: /tmp/other.svg: limit at byte 4\n"
        self.assertEqual(report_tool.parse_diagnostic(stderr, Path("/tmp/a.svg")), (None, None))

    def test_last_matching_diagnostic_wins(self) -> None:
        stderr = (
            "render-frame: /tmp/a.svg: limit at byte 1\n"
            "render-frame: /tmp/a.svg: path at byte 22\n"
        )
        self.assertEqual(report_tool.parse_diagnostic(stderr, Path("/tmp/a.svg")), ("path", 22))

    def test_missing_or_malformed_offsets_are_not_invented(self) -> None:
        cases = (
            "render-frame: /tmp/a.svg: boom\n",
            "render-frame: /tmp/a.svg: limit at byte -3\n",
            "render-frame: /tmp/a.svg: limit at byte 1.5\n",
            "render-frame: /tmp/a.svg: limit at byte abc\n",
            "",
        )
        for stderr in cases:
            self.assertEqual(
                report_tool.parse_diagnostic(stderr, Path("/tmp/a.svg")), (None, None), stderr
            )

    def test_failure_note_is_bounded_and_path_checked(self) -> None:
        note = report_tool.failure_note(
            "render-frame: /tmp/a.svg: refusing to overwrite the source\n", Path("/tmp/a.svg")
        )
        self.assertEqual(note, "refusing to overwrite the source")
        self.assertIsNone(
            report_tool.failure_note("render-frame: /tmp/b.svg: other\n", Path("/tmp/a.svg"))
        )

    def test_feature_specific_unsupported_maps_to_unsupported(self) -> None:
        self.assertEqual(
            report_tool.verdict_for("unsupported-fe-turbulence", 1, False), "unsupported"
        )

    def test_verdict_vocabulary_is_the_frozen_set(self) -> None:
        for category in ("invalid-argument", "none", "mystery", "malformed"):
            self.assertEqual(report_tool.verdict_for(category, 1, False), report_tool.ERROR)
        for category in ("limit", "xml", "number", "viewport", "path", "oom"):
            self.assertEqual(report_tool.verdict_for(category, 1, False), category)

    def test_success_and_timeout_verdicts(self) -> None:
        self.assertEqual(report_tool.verdict_for(None, 0, False), report_tool.SUCCESS)
        self.assertEqual(report_tool.verdict_for(None, 124, True), report_tool.TIMEOUT)
        self.assertEqual(report_tool.verdict_for(None, 1, False), report_tool.ERROR)

    def test_every_produced_verdict_is_in_the_frozen_set(self) -> None:
        produced = {
            report_tool.verdict_for(category, code, timed_out)
            for category in (None, "none", "invalid-argument", "xml", "limit", "mystery")
            for code in (0, 1)
            for timed_out in (False, True)
        }
        self.assertTrue(produced <= set(report_tool.REPORTED_VERDICTS))

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
    def test_digest_is_returned_when_it_matches(self) -> None:
        data = json.dumps([{"path": "a.svg", "bytes": 1}]).encode()
        digest = hashlib.sha256(data).hexdigest()
        self.assertEqual(report_tool.verify_manifest_digest(data, digest), digest)

    def test_digest_mismatch_aborts(self) -> None:
        with self.assertRaises(report_tool.ReportError):
            report_tool.verify_manifest_digest(b"[]")

    def test_missing_manifest_aborts(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            with self.assertRaises(report_tool.ReportError):
                report_tool.load_manifest(Path(tmp))

    def test_manifest_records_need_paths(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            corpus = Path(tmp)
            data = json.dumps([{"bytes": 4}]).encode()
            (corpus / "manifest.json").write_bytes(data)
            with self.assertRaises(report_tool.ReportError):
                report_tool.load_manifest(corpus, hashlib.sha256(data).hexdigest())


class MainGuardTests(unittest.TestCase):
    """AC-4: a bad manifest exits non-zero and writes no report."""

    def test_digest_mismatch_exits_non_zero_and_writes_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            corpus = base / "corpus"
            corpus.mkdir()
            (corpus / "manifest.json").write_text(json.dumps([{"path": "a.svg", "bytes": 1}]))
            binary = base / "fake-bin"
            binary.write_text("#!/bin/sh\nexit 1\n")
            out_json = base / "out.json"
            out_md = base / "out.md"
            code = report_tool.main(
                [
                    "--corpus", str(corpus),
                    "--binary", str(binary),
                    "--output-json", str(out_json),
                    "--output-markdown", str(out_md),
                ]
            )
            self.assertEqual(code, 2)
            self.assertFalse(out_json.exists())
            self.assertFalse(out_md.exists())

    def test_missing_binary_exits_non_zero_and_writes_nothing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            base = Path(tmp)
            out_json = base / "out.json"
            code = report_tool.main(
                [
                    "--corpus", str(base),
                    "--binary", str(base / "does-not-exist"),
                    "--output-json", str(out_json),
                    "--output-markdown", str(base / "out.md"),
                ]
            )
            self.assertEqual(code, 2)
            self.assertFalse(out_json.exists())


class _FakeRunner:
    """Deterministic stand-in for the render binary."""

    def __init__(self, table: dict[str, tuple[int, str, bool]]):
        self.table = table
        self.calls: list[list[str]] = []

    def __call__(self, argv, timeout):  # noqa: ANN001 - matches the runner protocol
        self.calls.append(list(argv))
        return self.table.get(Path(argv[2]).name, (0, "", False))


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

    def _report(self, tmp: str, runner: _FakeRunner, max_bytes: int = 1024) -> dict:
        return report_tool.build_report(
            self._entries(), self._corpus(tmp), Path("fake-bin"), "64x64", max_bytes, 5.0,
            Path(tmp) / "scratch.ppm", runner,
        )

    def test_histogram_and_skipped_accounting_cover_every_entry(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: files/bad.svg: limit at byte 0\n", False),
                }
            )
            report = self._report(tmp, runner)
            self.assertEqual(report["file_count"], 3)
            self.assertEqual(sum(report["histogram"].values()), 3)
            self.assertEqual(report["histogram"]["ok"], 1)
            self.assertEqual(report["histogram"]["limit"], 1)
            self.assertEqual(report["histogram"]["skipped"], 1)
            self.assertEqual(report["coverage"]["render_tested"], 2)
            self.assertEqual(report["coverage"]["not_render_tested"], 1)
            self.assertEqual(len(runner.calls), 2)

    def test_every_non_ok_record_explains_itself(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: files/bad.svg: refusing to load\n", False),
                }
            )
            report = self._report(tmp, runner)
            for item in report["files"]:
                if item["verdict"] == "ok":
                    self.assertIsNone(item["diagnostic_note"])
                    continue
                explained = (
                    item.get("diagnostic_category") is not None
                    or item.get("diagnostic_note") is not None
                )
                self.assertTrue(explained, item)
            failed = [item for item in report["files"] if item["verdict"] == report_tool.ERROR]
            self.assertEqual(len(failed), 1)
            self.assertEqual(failed[0]["diagnostic_note"], "refusing to load")

    def test_fake_binary_stderr_maps_to_a_verdict_and_offset(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (
                        1,
                        f"render-frame: {self._corpus(tmp) / 'files/bad.svg'}: unsupported at byte 7\n",
                        False,
                    ),
                }
            )
            report = self._report(tmp, runner)
            item = [i for i in report["files"] if i["id"] == "b"][0]
            self.assertEqual(item["verdict"], "unsupported")
            self.assertEqual(item["diagnostic_offset"], 7)
            self.assertEqual(item["exit_code"], 1)
            self.assertIsNotNone(item["excerpt"])

    def test_mismatched_path_diagnostic_is_not_attributed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner(
                {"bad.svg": (1, "render-frame: /somewhere/else.svg: limit at byte 3\n", False)}
            )
            report = self._report(tmp, runner)
            item = [i for i in report["files"] if i["id"] == "b"][0]
            self.assertEqual(item["verdict"], report_tool.ERROR)
            self.assertIsNone(item["diagnostic_offset"])

    def test_timeout_is_reported_with_a_note(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner({"ok.svg": (0, "", False), "bad.svg": (124, "", True)})
            report = self._report(tmp, runner)
            item = [i for i in report["files"] if i["id"] == "b"][0]
            self.assertEqual(item["verdict"], report_tool.TIMEOUT)
            self.assertIn("timeout", item["diagnostic_note"])

    def test_missing_source_is_reported_not_crashed(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            entry = [{"id": "x", "path": "files/gone.svg", "bytes": 10, "tier": "T1", "kind": "real"}]
            report = report_tool.build_report(
                entry, self._corpus(tmp), Path("fake-bin"), "64x64", 1024, 5.0,
                Path(tmp) / "scratch.ppm", _FakeRunner({}),
            )
            item = report["files"][0]
            self.assertEqual(item["verdict"], report_tool.ERROR)
            self.assertIn("missing", item["diagnostic_note"])

    def test_json_output_is_byte_identical_across_runs(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: files/bad.svg: xml at byte 3\n", False),
                }
            )
            outputs = []
            for run in range(2):
                report = self._report(tmp, runner)
                target = Path(tmp) / f"report-{run}.json"
                report_tool.write_json(target, report)
                outputs.append(target.read_bytes())
            self.assertEqual(outputs[0], outputs[1])
            self.assertTrue(outputs[0].endswith(b"\n"))

    def test_markdown_states_coverage_and_each_failure(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            runner = _FakeRunner(
                {
                    "ok.svg": (0, "", False),
                    "bad.svg": (1, "render-frame: files/bad.svg: limit at byte 0\n", False),
                }
            )
            markdown = report_tool.render_markdown(self._report(tmp, runner))
            self.assertIn("full-document render", markdown)
            self.assertIn("`limit`", markdown)
            self.assertIn("| 0 |", markdown)
            self.assertIn("render-tested: 2", markdown)
            self.assertIn("not render-tested: 1", markdown)
            self.assertIn("## Files that render", markdown)


if __name__ == "__main__":
    unittest.main()
