"""Focused contract tests for navigation replay validation and determinism."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DRIVER = ROOT / "app/test_driver/driver.py"


class ViewerNavigationDriverTests(unittest.TestCase):
    def test_navigation_replay_deterministic(self) -> None:
        with tempfile.TemporaryDirectory(dir=ROOT / ".cache") as raw:
            root = Path(raw)
            script = root / "script.jsonl"
            script.write_text("""{"command":"open","document_id":"nav"}
{"command":"navigate","action":"fit","x":0,"y":0,"value":0}
{"command":"navigate","action":"wheel","value":-120,"x":80,"y":60}
{"command":"index_progress","generation":2,"percent":37,"phase":"active","label":"Indexing"}
{"command":"screenshot"}
{"command":"accessibility"}
{"command":"quit"}
""", encoding="utf-8")
            outputs = [root / "one", root / "two"]
            for output in outputs:
                result = subprocess.run([sys.executable, str(DRIVER), "--navigation-replay",
                    "--script", str(script), "--output-dir", str(output), "--seed", "17"],
                    cwd=ROOT, text=True, capture_output=True, check=False)
                self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual({p.name: p.read_bytes() for p in outputs[0].iterdir()},
                             {p.name: p.read_bytes() for p in outputs[1].iterdir()})
            semantic = json.loads((outputs[0] / "semantics-0000.json").read_text())
            self.assertEqual(semantic["children"][1]["value"], 37)
            self.assertEqual(semantic["children"][2]["enabled"], True)


if __name__ == "__main__":
    unittest.main()
