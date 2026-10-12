"""render-frame prints one note line per ignored construct kind (WO-CORPUS-01)."""
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
BINARY = Path(os.environ.get("VEXEL_SVGWORKBENCH", ROOT / "build/mac-debug/bin/svgworkbench"))

SOURCE = (
    '<?xml version="1.0"?>\n<!DOCTYPE svg [<!ENTITY e "x">]>\n'
    '<svg width="8" height="8" data-a="1"><meta charset="utf-8"/>'
    '<rect width="4" height="4" data-b="2" style="writing-mode:tb"/></svg>'
)


class RenderFrameIgnoredNoteTests(unittest.TestCase):
    def run_cli(self, text):
        self.assertTrue(BINARY.is_file(), f"missing {BINARY}")
        with tempfile.TemporaryDirectory(prefix="vexel-ignored-") as temp:
            source = Path(temp) / "case.svg"
            source.write_text(text, encoding="utf-8")
            run = subprocess.run(
                [str(BINARY), "--render-frame", str(source), "--size", "16x16",
                 "--out", str(Path(temp) / "out.ppm")], capture_output=True, text=True)
            return run, str(source)

    def test_one_line_per_kind_on_success(self):
        run, path = self.run_cli(SOURCE)
        self.assertEqual(run.returncode, 0, run.stderr)
        offset = SOURCE.index
        self.assertEqual(run.stderr.splitlines(), [
            f"render-frame: {path}: note: ignored unknown-attribute x2, first at byte {offset('data-a')}",
            f"render-frame: {path}: note: ignored meta x1, first at byte {offset('<meta')}",
            f"render-frame: {path}: note: ignored doctype x1, first at byte {offset('<!DOCTYPE')}",
            f"render-frame: {path}: note: ignored writing-mode x1, first at byte {offset('writing-mode')}",
        ])

    def test_clean_document_prints_nothing(self):
        run, _ = self.run_cli('<svg width="8" height="8"><rect width="4" height="4"/></svg>')
        self.assertEqual((run.returncode, run.stderr), (0, ""))

    def test_failure_keeps_single_diagnostic(self):
        run, path = self.run_cli('<svg width="8" height="8"><use href="#missing" data-a="1"/></svg>')
        self.assertNotEqual(run.returncode, 0)
        self.assertNotIn("note:", run.stderr)


if __name__ == "__main__":
    unittest.main()
