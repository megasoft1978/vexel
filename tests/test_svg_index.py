"""Focused incremental index contract checks."""

from __future__ import annotations

import json
import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TEST_BINARY = ROOT / "build/mac-debug/bin/svg_index_test"


class SvgIndexTests(unittest.TestCase):
    def test_index_contract_binary(self) -> None:
        self.assertTrue(TEST_BINARY.is_file(), "run ./dev build first")
        result = subprocess.run([str(TEST_BINARY)], cwd=ROOT, capture_output=True,
                                text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)

    def test_frozen_corpus_incremental_token_equivalence(self) -> None:
        manifest = json.loads((ROOT / "bench/corpus/manifest.json").read_text(encoding="utf-8"))
        fixture_paths = [str(ROOT / item["path"]) for item in manifest["fixtures"]]
        self.assertEqual(len(fixture_paths), manifest["fixture_count"])
        result = subprocess.run([str(TEST_BINARY), "--equivalence", *fixture_paths],
                                cwd=ROOT, capture_output=True, text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        evidence = json.loads(result.stdout)
        self.assertEqual(evidence["fixture_count"], manifest["fixture_count"])


if __name__ == "__main__":
    unittest.main()
