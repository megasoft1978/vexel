"""Offline validation for the immutable independent SVG render baseline."""

from __future__ import annotations

import json
import hashlib
import sys
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "bench/generate"))
import render_oracle


class GoldenOracleTests(unittest.TestCase):
    def test_committed_oracle_lock_verifies_offline(self) -> None:
        render_oracle.validate_lock()
        lock = json.loads((ROOT / "tests/golden/manifest.lock").read_text(encoding="utf-8"))
        self.assertEqual(len(lock["fixtures"]), 24)
        self.assertEqual(lock["renderer"]["version"], "2.62.3")
        self.assertIn("LGPL-2.1-or-later", lock["renderer"]["license"])
        self.assertEqual(lock["raster"], {"width": 256, "height": 256, "format": "PNG"})

    def test_render_command_rechecks_without_rewriting_immutable_pngs(self) -> None:
        lock_path = ROOT / "tests/golden/manifest.lock"
        lock = json.loads(lock_path.read_text(encoding="utf-8"))
        pngs = [ROOT / fixture["png_path"] for fixture in lock["fixtures"]]
        before = [hashlib.sha256(path.read_bytes()).hexdigest() for path in pngs]
        with tempfile.TemporaryDirectory() as name:
            evidence = Path(name) / "render.txt"
            version_output = lock["renderer"]["version_output"] + "\n"
            with patch.object(render_oracle, "EVIDENCE", evidence), patch.object(
                    render_oracle.subprocess, "run",
                    return_value=SimpleNamespace(stdout=version_output)):
                render_oracle.render("rsvg-convert", "2.62.3")
            self.assertIn("existing immutable baseline verified", evidence.read_text())
        after = [hashlib.sha256(path.read_bytes()).hexdigest() for path in pngs]
        self.assertEqual(before, after)


if __name__ == "__main__":
    unittest.main()
