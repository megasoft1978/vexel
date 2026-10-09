"""Focused Python entry point for the native spatial/cache contract test."""

from __future__ import annotations

import subprocess
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / "build/mac-debug/bin/spatial_cache_test"


class SpatialCacheTests(unittest.TestCase):
    def test_spatial_cache_contract_binary(self) -> None:
        self.assertTrue(BINARY.is_file(), "run ./dev build first")
        result = subprocess.run([str(BINARY)], cwd=ROOT, capture_output=True,
                                text=True, check=False)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("spatial and tile-cache deterministic checks passed", result.stdout)
        self.assertEqual(result.stderr, "")


if __name__ == "__main__":
    unittest.main()
