"""Safety and reproducibility checks for the offline benchmark harness."""

from __future__ import annotations

import csv
import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest import mock

ROOT = Path(__file__).resolve().parents[1]
SCRIPT = ROOT / "bench/harness/compare.py"
SPEC = importlib.util.spec_from_file_location("compare", SCRIPT)
compare = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = compare
SPEC.loader.exec_module(compare)


class BenchmarkHarnessTests(unittest.TestCase):
    def setUp(self):
        self.cache = ROOT / ".cache/m0-03-tests"
        self.cache.mkdir(parents=True, exist_ok=True)
        self.meta, self.records = compare.load_corpus()
        self.fixture = self.records["shapes-basic"]
        self.adapter = {"id": "fake", "path": sys.executable,
                        "render_argv": [sys.executable, "-c", "pass", "{output}"],
                        "timeout_seconds": 1.0}

    def tearDown(self):
        import shutil
        shutil.rmtree(self.cache, ignore_errors=True)

    def test_corpus_hashes_are_pinned(self):
        meta, records = compare.load_corpus()
        self.assertEqual(meta["fixture_count"], 24)
        self.assertEqual(len(records), 24)
        for record in records.values():
            self.assertEqual(compare.sha(Path(record["path_abs"]).read_bytes()), record["sha256"])

    def test_png_validator_checks_dimensions_and_crc(self):
        png = compare.fake_png()
        self.assertEqual(compare.png_dimensions(png), (256, 256))
        with self.assertRaises(compare.BenchError):
            compare.png_dimensions(png[:-3] + b"bad")
        with self.assertRaises(compare.BenchError):
            compare.png_dimensions(b"not a png")

    def test_command_config_rejects_shell_strings_and_unknown_placeholders(self):
        for bad in ("renderer {input}", ["renderer", "{unknown}"], ["renderer", "{input"]):
            with self.subTest(bad=bad), self.assertRaises(compare.BenchError):
                compare.validate_command(bad)

    def test_shell_metacharacters_remain_one_argv_value(self):
        marker = self.cache / "shell-injection-marker"
        payload = f"; touch {marker}"
        result = compare.process([sys.executable, "-c", "import sys; print(sys.argv[1])", payload], 2.0)
        self.assertEqual(result[0], 0)
        self.assertIn(payload, result[1].decode())
        self.assertFalse(marker.exists())

    def test_fixture_tampering_is_detected_in_isolated_snapshot(self):
        import shutil
        with tempfile.TemporaryDirectory(dir=self.cache) as folder:
            root = Path(folder)
            shutil.copytree(ROOT / "bench/corpus", root / "bench/corpus")
            fixture = root / "bench/corpus/fixtures/shapes-basic.svg"
            fixture.write_bytes(fixture.read_bytes() + b"tamper")
            with mock.patch.object(compare, "ROOT", root), \
                 mock.patch.object(compare, "CORPUS", root / "bench/corpus"), \
                 mock.patch.object(compare, "MANIFEST", root / "bench/corpus/manifest.json"), \
                 mock.patch.object(compare, "MANIFEST_SHA", root / "bench/corpus/manifest.sha256"), \
                 self.assertRaises(compare.BenchError):
                compare.load_corpus()

    def test_config_schema_rejects_shell_command_field(self):
        with tempfile.TemporaryDirectory(dir=self.cache) as folder:
            path = Path(folder) / "adapter.json"
            path.write_text(json.dumps({"schema_version": 1, "adapters": [{"id": "x", "path": "/x",
                             "version_argv": ["--version"], "render_argv": "renderer {input}",
                             "timeout_seconds": 1}]}))
            with self.assertRaises(compare.BenchError):
                compare.adapter_list(path, False)

    def test_output_path_must_stay_under_repo_cache(self):
        with self.assertRaises(compare.BenchError):
            compare.safe_cache_target(str(ROOT / "outside.png"), self.cache)
        with self.assertRaises(compare.BenchError):
            compare.safe_cache_target("/tmp/unsafe.png", self.cache)
        link = self.cache / "escape.png"
        link.symlink_to(ROOT.parent / "outside.png")
        try:
            with self.assertRaises(compare.BenchError):
                compare.safe_cache_target(str(link), self.cache)
        finally:
            link.unlink(missing_ok=True)
        cache_link = ROOT / ".cache" / "m0-03-cache-escape-test"
        cache_link.symlink_to(ROOT.parent, target_is_directory=True)
        try:
            with self.assertRaises(compare.BenchError):
                compare.safe_cache_target(str(cache_link / "escape.png"), cache_link)
        finally:
            cache_link.unlink(missing_ok=True)

    def test_report_paths_and_adapter_ids_are_confined(self):
        with self.assertRaises(compare.BenchError):
            compare.safe_result_path("/tmp/outside.json")
        link = ROOT / "bench/results" / "m0-03-result-escape-test.json"
        link.symlink_to(ROOT.parent / "outside.json")
        try:
            with self.assertRaises(compare.BenchError):
                compare.safe_result_path(str(link))
        finally:
            link.unlink(missing_ok=True)
        with tempfile.TemporaryDirectory(dir=self.cache) as folder:
            config = Path(folder) / "adapter.json"
            config.write_text(json.dumps({"schema_version": 1, "adapters": [{
                "id": "../escape", "path": sys.executable,
                "version_argv": ["--version"], "render_argv": ["renderer", "{input}"],
                "timeout_seconds": 1.0}]}))
            with self.assertRaisesRegex(compare.BenchError, "filename component"):
                compare.adapter_list(config, False)

    def _run_with_process(self, name, code, error=None, write_output=True, valid=True):
        out = self.cache / "outputs/fake/shapes-basic/cold-process-1.png"
        def fake_process(argv, timeout):
            if write_output:
                data = compare.fake_png()
                if not valid:
                    data = b"malformed"
                Path(argv[-1]).write_bytes(data)
            return code, b"out", b"err", .01, error, None
        with mock.patch.object(compare, "process", side_effect=fake_process):
            return compare.run_one(self.adapter, self.fixture, self.cache, "cold-process", 1)

    def test_timeout_is_failed_trial(self):
        trial = self._run_with_process("timeout", None, "timeout")
        self.assertEqual(trial["status"], "failed")
        self.assertEqual(trial["error"], "timeout")

    def test_nonzero_exit_is_failed_trial(self):
        trial = self._run_with_process("nonzero", 9)
        self.assertEqual(trial["status"], "failed")
        self.assertEqual(trial["exit_code"], 9)

    def test_missing_output_is_failed_trial(self):
        trial = self._run_with_process("missing", 0, write_output=False)
        self.assertEqual(trial["status"], "failed")
        self.assertIn("missing", trial["error"])

    def test_malformed_output_is_failed_trial(self):
        trial = self._run_with_process("malformed", 0, valid=False)
        self.assertEqual(trial["status"], "failed")
        self.assertIn("PNG", trial["error"])

    def test_median_and_nearest_rank_p95_need_five_valid_trials(self):
        trials = [{"status": "passed", "elapsed_seconds": n, "peak_rss_bytes": None} for n in range(1, 6)]
        self.assertEqual(compare.stats(trials, "elapsed_seconds"),
                         {"valid_count": 5, "median": 3, "p95_nearest_rank": 5})
        self.assertIsNone(compare.stats(trials[:4], "elapsed_seconds")["median"])

    def test_offline_fake_adapter_runs_five_cold_and_warm_per_fixture(self):
        with tempfile.TemporaryDirectory(dir=self.cache) as folder:
            json_path, csv_path = Path(folder) / "fake.json", Path(folder) / "fake.csv"
            result = subprocess.run([sys.executable, str(SCRIPT), "--self-test-adapter", "--json", str(json_path), "--csv", str(csv_path)],
                                    cwd=ROOT, capture_output=True, text=True, timeout=60)
            self.assertEqual(result.returncode, 0, result.stderr)
            report = json.loads(json_path.read_text())
            self.assertEqual(len(report["trials"]), 30)
            self.assertTrue(all(t["status"] == "passed" for t in report["trials"]))
            for fixture in compare.FIXTURES:
                for condition in ("cold-process", "warm-repeat"):
                    group = [t for t in report["trials"] if t["fixture_id"] == fixture and t["condition"] == condition]
                    self.assertEqual(len(group), 5)
                    self.assertEqual(len({t["output_sha256"] for t in group}), 1)
            with csv_path.open(newline="") as stream:
                rows = list(csv.DictReader(stream))
            raw_rows = [r for r in rows if r["record_type"] == "trial"]
            summary_rows = [r for r in rows if r["record_type"] == "summary"]
            self.assertEqual([r["trial_id"] for r in raw_rows], [t["trial_id"] for t in report["trials"]])
            self.assertEqual([r["output_sha256"] for r in raw_rows], [t["output_sha256"] for t in report["trials"]])
            self.assertEqual(len(summary_rows), len(report["summary"]))
            for row, summary in zip(summary_rows, report["summary"]):
                self.assertEqual(float(row["latency_median"]), summary["latency_seconds"]["median"])
            compare.verify(json_path, csv_path)

    def test_verify_detects_json_csv_fixture_and_output_tampering(self):
        with tempfile.TemporaryDirectory(dir=self.cache) as folder:
            folder = Path(folder)
            output = self.cache / "outputs/tamper.png"
            output.parent.mkdir(parents=True, exist_ok=True)
            data = compare.fake_png()
            output.write_bytes(data)
            trial = {"trial_id": "t", "adapter_id": "a", "fixture_id": "shapes-basic",
                     "fixture_sha256": self.fixture["sha256"], "condition": "cold-process", "trial_index": 1,
                     "status": "passed", "exit_code": 0, "elapsed_seconds": .1, "peak_rss_bytes": None,
                     "output_path": str(output.relative_to(ROOT)), "output_size": len(data), "output_sha256": compare.sha(data),
                     "stdout_preview": "", "stdout_sha256": compare.sha(b""), "stderr_preview": "", "stderr_sha256": compare.sha(b""),
                     "argv": ["fake"], "error": None}
            report = {"schema_version": 1, "work_order": "WO-M0-03", "fixtures": [{"fixture_id": "shapes-basic", "fixture_sha256": self.fixture["sha256"], "source_sha256": self.fixture["source_sha256"]}],
                      "trials": [trial], "summary": []}
            compare.install_report(report, folder / "report.json", folder / "report.csv", [trial])
            jpath, cpath = folder / "report.json", folder / "report.csv"
            compare.verify(jpath, cpath)
            original = jpath.read_bytes()
            jpath.write_bytes(original + b" ")
            with self.assertRaises(compare.BenchError):
                compare.verify(jpath, cpath)
            jpath.write_bytes(original)
            oldcsv = cpath.read_bytes()
            cpath.write_bytes(oldcsv + b"tamper")
            with self.assertRaises(compare.BenchError):
                compare.verify(jpath, cpath)
            cpath.write_bytes(oldcsv)
            output.write_bytes(b"tampered")
            with self.assertRaises(compare.BenchError):
                compare.verify(jpath, cpath)
            output.write_bytes(data)
            altered = json.loads(original)
            altered["fixtures"][0]["fixture_sha256"] = "0" * 64
            altered["report_sha256"] = ""
            altered["report_sha256"] = compare.sha(compare.canonical_json(altered))
            jpath.write_bytes(compare.canonical_json(altered))
            with self.assertRaises(compare.BenchError):
                compare.verify(jpath, cpath)
            output.unlink(missing_ok=True)


if __name__ == "__main__":
    unittest.main()
