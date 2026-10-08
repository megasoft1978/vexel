"""Determinism, bounds, path-safety, and atomicity checks for SVG generation."""

from __future__ import annotations

import hashlib
import json
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "bench/generate"))
import stress_svg


class StressGeneratorTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp = tempfile.TemporaryDirectory()
        self.root = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def build(self, name: str, family: str, **options: int) -> tuple[Path, dict[str, object]]:
        path = self.root / name
        controls = {"objects": 20, "commands": 30, "depth": 4, "image_bytes": 64}
        aliases = {"path_commands": "commands", "image_payload_bytes": "image_bytes",
                   "nested_groups": "depth"}
        for key, value in options.items():
            if key == "images":
                if value == 0:
                    controls["image_bytes"] = 0
            else:
                controls[aliases.get(key, key)] = value
        result = stress_svg.generate(path, family=family, seed=12648430, size=4096,
                                    **controls)
        return path, result

    def test_all_families_have_expected_semantic_controls_and_valid_svg(self) -> None:
        expected = {
            "object-heavy": {"objects": 12, "path_commands": 0, "images": 0},
            "path-heavy": {"objects": 0, "path_commands": 13, "images": 0},
            "nested-depth": {"objects": 0, "path_commands": 0, "nested_groups": 7},
            "embedded-image": {"image_payload_bytes": 97, "images": 1},
        }
        for family, controls in expected.items():
            with self.subTest(family=family):
                path, result = self.build(f"{family}.svg", family, **controls)
                self.assertEqual(path.stat().st_size, 4096)
                self.assertEqual(result["bytes"], 4096)
                self.assertEqual(hashlib.sha256(path.read_bytes()).hexdigest(), result["sha256"])
                self.assertIn(b"<svg", path.read_bytes())
                for key, value in controls.items():
                    self.assertEqual(result["counts"][key], value)
        mixed, result = self.build("mixed.svg", "mixed", objects=9, path_commands=12,
                                   nested_groups=3, image_payload_bytes=24)
        self.assertEqual(mixed.stat().st_size, 4096)
        self.assertEqual(result["counts"], {"objects": 3, "path_commands": 4,
                                             "nested_groups": 3, "images": 1,
                                             "image_payload_bytes": 12})

    def test_seed_and_controls_reproduce_exact_bytes(self) -> None:
        first, _ = self.build("first.svg", "object-heavy")
        second, _ = self.build("second.svg", "object-heavy")
        self.assertEqual(first.read_bytes(), second.read_bytes())
        repeated = stress_svg.generate(first, family="object-heavy", seed=12648430,
                                       size=4096, objects=20, commands=30,
                                       depth=4, image_bytes=64)
        self.assertEqual(repeated["sha256"], hashlib.sha256(first.read_bytes()).hexdigest())
        third = self.root / "different-seed.svg"
        stress_svg.generate(third, family="object-heavy", seed=12648431, size=4096, objects=20)
        self.assertNotEqual(first.read_bytes(), third.read_bytes())

    def test_exact_size_and_independent_semantic_count(self) -> None:
        path = self.root / "large.svg"
        result = stress_svg.generate(path, family="path-heavy", seed=7, size=100_000,
                                    commands=17, objects=0, depth=0, image_bytes=0)
        self.assertEqual(path.stat().st_size, 100_000)
        self.assertEqual(result["counts"]["path_commands"], 17)

    def test_invalid_and_over_limit_options_create_no_destination(self) -> None:
        cases = ({"family": "unknown", "seed": 1, "size": 4096},
                 {"family": "object-heavy", "seed": -1, "size": 4096},
                 {"family": "object-heavy", "seed": 1, "size": stress_svg.MAX_BYTES + 1},
                 {"family": "object-heavy", "seed": 1, "size": 4096, "objects": stress_svg.MAX_OBJECTS + 1},
                 {"family": "path-heavy", "seed": 1, "size": 4096, "commands": stress_svg.MAX_PATH_COMMANDS + 1},
                 {"family": "nested-depth", "seed": 1, "size": 4096, "depth": stress_svg.MAX_DEPTH + 1},
                 {"family": "embedded-image", "seed": 1, "size": 4096, "image_bytes": stress_svg.MAX_IMAGE_BYTES + 1})
        for index, options in enumerate(cases):
            path = self.root / f"invalid-{index}.svg"
            with self.subTest(options=options), self.assertRaises(ValueError):
                stress_svg.generate(path, **options)
            self.assertFalse(path.exists())

    def test_semantic_overflow_and_existing_destination_are_atomic(self) -> None:
        path = self.root / "overflow.svg"
        with self.assertRaises(ValueError):
            stress_svg.generate(path, family="path-heavy", seed=1, size=128,
                                commands=100, objects=0, depth=0, image_bytes=0)
        self.assertFalse(path.exists())
        path.write_bytes(b"preserve")
        with self.assertRaises(ValueError):
            stress_svg.generate(path, family="object-heavy", seed=1, size=4096)
        self.assertEqual(path.read_bytes(), b"preserve")

    def test_path_traversal_names_are_rejected_and_tempfiles_cleaned(self) -> None:
        path = self.root / ".." / "outside.svg"
        with self.assertRaises(ValueError):
            stress_svg.generate(path, family="object-heavy", seed=1, size=4096)
        self.assertEqual(list(self.root.glob(".stress-svg-*")), [])

    def test_failed_atomic_install_removes_temporary_file(self) -> None:
        target = self.root / "claimed.svg"
        target.write_bytes(b"winner")
        with self.assertRaises(ValueError):
            stress_svg.generate(target, family="object-heavy", seed=1, size=4096)
        self.assertEqual(list(self.root.glob(".stress-svg-*")), [])
        self.assertEqual(target.read_bytes(), b"winner")

    def test_atomic_link_failure_cleans_tempfile_and_leaves_no_destination(self) -> None:
        target = self.root / "link-failure.svg"
        with patch.object(stress_svg.os, "link", side_effect=OSError("injected install failure")):
            with self.assertRaisesRegex(OSError, "injected install failure"):
                stress_svg.generate(target, family="object-heavy", seed=1, size=4096, objects=1)
        self.assertFalse(target.exists())
        self.assertEqual(list(self.root.glob(".stress-svg-*")), [])

    def test_tampered_report_artifact_is_rejected_without_rewriting_input(self) -> None:
        artifact = self.root / "tampered.svg"
        artifact.write_bytes(b"tampered bytes")
        report = self.root / "report.json"
        report.write_text(json.dumps({"artifacts": [{"path": str(artifact), "bytes": 14,
                                                       "sha256": hashlib.sha256(b"original bytes").hexdigest()}]}))
        before = artifact.read_bytes()
        with self.assertRaisesRegex(ValueError, "hash mismatch"):
            stress_svg.verify_report(report)
        self.assertEqual(artifact.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()
