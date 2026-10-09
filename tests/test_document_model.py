"""Source-backed tree and stale-handle contract checks."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import unittest


ROOT = Path(__file__).resolve().parents[1]
EXECUTABLE = ROOT / "build/mac-debug/bin/document_model_test"
EVIDENCE = ROOT / "artifacts/WO-M5-01/handle-matrix.json"


class DocumentModelTests(unittest.TestCase):
    def test_native_handle_matrix_and_source_preservation(self) -> None:
        if not EXECUTABLE.exists():
            raise RuntimeError("build document_model_test with ./dev build first")
        result = subprocess.run([str(EXECUTABLE)], cwd=ROOT, text=True,
                                capture_output=True, check=False, timeout=10)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        record = json.loads(result.stdout)
        self.assertEqual(record["stable_handles"], True)
        self.assertGreaterEqual(record["stale_rejections"], 6)
        self.assertEqual(record["foreign_rejections"], 1)
        self.assertEqual(record["source_unchanged"], True)
        self.assertEqual(record["exact_ranges"], True)
        self.assertEqual(record["malformed_xml"], True)
        self.assertEqual(record["input_limit"], True)
        self.assertEqual(record["token_storage_limit"], True)
        EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
        EVIDENCE.write_text(json.dumps(record, indent=2, sort_keys=True) + "\n")


if __name__ == "__main__":
    unittest.main()
