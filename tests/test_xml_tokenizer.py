"""Focused Python driver for the tokenizer's compiled API contract tests."""
import json
from pathlib import Path
import subprocess
import unittest

from scripts.xml_fuzz import confined_output_path

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "build/mac-debug/bin/xml_tokenizer_test"


class XmlTokenizerTests(unittest.TestCase):
    def test_fuzz_output_is_confined_to_task_cache_or_artifacts(self):
        self.assertEqual(confined_output_path(ROOT, ".cache/WO-M2-02/fuzz.json"),
                         ROOT / ".cache/WO-M2-02/fuzz.json")
        self.assertEqual(confined_output_path(ROOT, "artifacts/WO-M2-02/fuzz.json"),
                         ROOT / "artifacts/WO-M2-02/fuzz.json")
        for path in ("../outside.json", "/tmp/outside.json"):
            with self.subTest(path=path), self.assertRaises(ValueError):
                confined_output_path(ROOT, path)

    def test_compiled_token_spans_errors_limits_and_source_retention(self):
        self.assertTrue(BINARY.is_file(), "run ./dev build before the focused API tests")
        run = subprocess.run([str(BINARY)], cwd=ROOT, check=True, capture_output=True, text=True)
        self.assertIn("xml tokenizer focused C checks passed", run.stdout)

    def test_deterministic_malformed_generation_count(self):
        run = subprocess.run([str(BINARY)], cwd=ROOT, check=True, capture_output=True, text=True)
        count = int(run.stdout.rsplit("=", 1)[1].strip())
        self.assertGreaterEqual(count, 100_000)

if __name__ == "__main__":
    unittest.main()
