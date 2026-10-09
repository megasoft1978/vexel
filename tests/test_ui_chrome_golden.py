"""Golden checks for the retained editor chrome renderer."""

from __future__ import annotations

import subprocess
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


class UiChromeGoldenTests(unittest.TestCase):
    def test_dark_and_light_editor_chrome(self) -> None:
        result = subprocess.run(
            [sys.executable, str(ROOT / "tests/ui/verify_chrome.py")],
            cwd=ROOT,
            check=False,
            capture_output=True,
            text=True,
            timeout=60,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn("dark, light (1280x800)", result.stdout)


if __name__ == "__main__":
    unittest.main()
