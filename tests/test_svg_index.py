"""Focused incremental index contract checks."""

from __future__ import annotations

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


if __name__ == "__main__":
    unittest.main()
