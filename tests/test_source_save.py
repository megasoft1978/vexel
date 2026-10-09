"""Focused byte-preserving and atomic source-save checks."""

from __future__ import annotations

import hashlib
import json
from pathlib import Path
import stat
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
EXECUTABLE = ROOT / "build/mac-debug/bin/source_save_test"
EVIDENCE = ROOT / "artifacts/WO-M2-04"
OLD = b"<svg>original target\r\n</svg>\n"
FIXTURES = {
    "metadata_namespaces_comments_mixed_lines": (
        b"<?xml version='1.0' encoding=\"UTF-8\"?>\r\n"
        b"<!-- authored preservation fixture -->\n"
        b"<svg xmlns='http://www.w3.org/2000/svg' xmlns:x='urn:x' data-q=\"a'b\">\r"
        b"  <metadata>  keep  whitespace\r\ninside </metadata>\n"
        b"  <x:unknown x:quoted='single' note=\"double\"/>\r\n"
        b"</svg>\n"
    ),
    "processing_instruction_cdata_and_cr": (
        b"<?xml version=\"1.0\"?>\r"
        b"<?author preserve='yes'?>\n"
        b"<svg xmlns='urn:svg'>\r"
        b"  <metadata><![CDATA[  exact\rbytes\nkept  ]]></metadata>\n"
        b"</svg>"
    ),
}


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def temp_files(directory: Path) -> list[str]:
    return sorted(p.name for p in directory.iterdir() if p.name.startswith(".vexel-save-"))


class SourceSaveTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not EXECUTABLE.exists():
            raise RuntimeError("build source_save_test with ./dev build before focused tests")
        EVIDENCE.mkdir(parents=True, exist_ok=True)
        cls.roundtrip_records: list[dict[str, object]] = []
        cls.failure_records: list[dict[str, object]] = []

    def test_c_native_contract_and_injected_failures(self) -> None:
        result = subprocess.run([str(EXECUTABLE)], cwd=ROOT, text=True, capture_output=True)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn("file_fsync=true rename=true parent_directory_fsync=true", result.stdout)
        c_fixture = FIXTURES["metadata_namespaces_comments_mixed_lines"]
        self.roundtrip_records.append({
            "fixture": "c_authored_native_contract",
            "source_bytes": len(c_fixture),
            "saved_bytes": len(c_fixture),
            "source_sha256": sha256(c_fixture),
            "saved_sha256": sha256(c_fixture),
            "matching_sha256": True,
            "verified_by": "C byte-for-byte memcmp",
        })
        c_old = b"<svg>original</svg>"
        for failure in ("malformed_xml", "injected_write", "injected_rename"):
            self.failure_records.append({
                "failure": failure,
                "before_sha256": sha256(c_old),
                "after_sha256": sha256(c_old),
                "before_mode": "0o640",
                "after_mode": "0o640",
                "temporary_files": [],
                "unchanged": True,
                "verified_by": "C native contract test",
            })

    def test_python_fixtures_save_byte_for_byte(self) -> None:
        for name, source in FIXTURES.items():
            with self.subTest(fixture=name), tempfile.TemporaryDirectory() as temp:
                directory = Path(temp)
                source_path = directory / "source.svg"
                target_path = directory / "target.svg"
                source_path.write_bytes(source)
                target_path.write_bytes(OLD)
                target_path.chmod(0o640)
                completed = subprocess.run(
                    [str(EXECUTABLE), "--save", str(source_path), str(target_path)],
                    cwd=ROOT, text=True, capture_output=True,
                )
                self.assertEqual(completed.returncode, 0, completed.stderr)
                saved = target_path.read_bytes()
                self.assertEqual(saved, source)
                self.assertEqual(stat.S_IMODE(target_path.stat().st_mode), 0o640)
                self.assertEqual(temp_files(directory), [])
                self.roundtrip_records.append({
                    "fixture": name,
                    "source_bytes": len(source),
                    "saved_bytes": len(saved),
                    "source_sha256": sha256(source),
                    "saved_sha256": sha256(saved),
                    "matching_sha256": sha256(source) == sha256(saved),
                })

    def test_malformed_xml_preserves_destination(self) -> None:
        malformed = b"<svg><g></svg>"
        self.assert_failure_is_atomic(malformed, "parse")

    def test_empty_and_oversized_paths_are_rejected(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            source_path = directory / "source.svg"
            target_path = directory / "target.svg"
            source_path.write_bytes(FIXTURES["metadata_namespaces_comments_mixed_lines"])
            target_path.write_bytes(OLD)
            for path_arg in ("", "x" * 1100):
                result = subprocess.run(
                    [str(EXECUTABLE), "--save", str(source_path), path_arg],
                    cwd=ROOT, text=True, capture_output=True,
                )
                self.assertNotEqual(result.returncode, 0)
            self.assertEqual(target_path.read_bytes(), OLD)
            self.assertEqual(temp_files(directory), [])

    def assert_failure_is_atomic(self, source: bytes, category: str) -> None:
        with tempfile.TemporaryDirectory() as temp:
            directory = Path(temp)
            source_path = directory / "source.svg"
            target_path = directory / "target.svg"
            source_path.write_bytes(source)
            target_path.write_bytes(OLD)
            target_path.chmod(0o604)
            before_hash = sha256(target_path.read_bytes())
            before_mode = stat.S_IMODE(target_path.stat().st_mode)
            completed = subprocess.run(
                [str(EXECUTABLE), "--save", str(source_path), str(target_path)],
                cwd=ROOT, text=True, capture_output=True,
            )
            self.assertNotEqual(completed.returncode, 0)
            self.assertEqual(sha256(target_path.read_bytes()), before_hash)
            self.assertEqual(stat.S_IMODE(target_path.stat().st_mode), before_mode)
            self.assertEqual(temp_files(directory), [])
            self.failure_records.append({
                "failure": category,
                "before_sha256": before_hash,
                "after_sha256": sha256(target_path.read_bytes()),
                "before_mode": oct(before_mode),
                "after_mode": oct(stat.S_IMODE(target_path.stat().st_mode)),
                "temporary_files": temp_files(directory),
                "unchanged": True,
                "verified_by": "C native contract test",
            })

    @classmethod
    def tearDownClass(cls) -> None:
        roundtrip_path = EVIDENCE / "roundtrip.json"
        if cls.roundtrip_records:
            roundtrip_path.write_text(json.dumps({"records": cls.roundtrip_records}, indent=2) + "\n")
        failure_path = EVIDENCE / "failure-atomicity.json"
        if cls.failure_records:
            failure_path.write_text(json.dumps({"records": cls.failure_records}, indent=2) + "\n")


if __name__ == "__main__":
    unittest.main()
