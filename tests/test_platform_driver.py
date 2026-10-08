"""Focused acceptance coverage for the native platform driver contract."""

from __future__ import annotations

import json
import subprocess
import sys
import tempfile
import subprocess
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "app" / "test_driver"))
import driver  # noqa: E402


class PlatformDriverTests(unittest.TestCase):
    def setUp(self) -> None:
        self.expected = ["open", "resize", "scale", "key_down", "key_up",
                         "pointer_move", "pointer_down", "pointer_up", "close_request"]
        self.events = [
            {"type": "open"},
            {"type": "resize", "pixel_width": 640, "pixel_height": 480, "backing_scale": 2.0},
            {"type": "scale", "pixel_width": 640, "pixel_height": 480, "backing_scale": 2.0},
            {"type": "key_down", "key_code": 0, "modifiers": 0, "text": "a"},
            {"type": "key_up", "key_code": 0, "modifiers": 0, "text": ""},
            {"type": "pointer_move", "x": 12, "y": 18},
            {"type": "pointer_down", "x": 12, "y": 18, "button": 0},
            {"type": "pointer_up", "x": 12, "y": 18, "button": 0},
            {"type": "close_request"},
        ]

    def test_valid_lifecycle_replay(self) -> None:
        commands = driver.read_script(Path(__file__).parent / "ui" / "native-lifecycle.jsonl")
        self.assertEqual([item["command"] for item in commands],
                         ["open", "resize", "key", "key", "pointer", "pointer", "pointer", "quit"])
        driver.validate_normalized_events(self.events, self.expected)
        self.assertEqual(driver.native_command_arguments(commands), [
            "--open", "native-smoke", "--resize", "640", "480",
            "--key", "down", "a", "--key", "up", "a",
            "--pointer", "move", "12", "18", "--pointer", "down", "12", "18",
            "--pointer", "up", "12", "18", "--quit",
        ])

    def test_native_translation_uses_scenario_values(self) -> None:
        commands = driver.read_script(Path(__file__).parent / "ui" / "native-lifecycle.jsonl")
        commands[0]["document_id"] = "custom-window-title"
        commands[1]["width"], commands[1]["height"] = 800, 600
        for command in commands:
            if command["command"] == "pointer":
                command["x"], command["y"] = 20, 30
        translated = driver.native_command_arguments(commands)
        self.assertEqual(translated[:5], ["--open", "custom-window-title", "--resize", "800", "600"])
        self.assertEqual(translated[11:24], [
            "--pointer", "move", "20", "30", "--pointer", "down", "20", "30",
            "--pointer", "up", "20", "30", "--quit",
        ])

    def test_resize_and_scale_payloads(self) -> None:
        self.assertEqual(self.events[1]["pixel_width"], 640)
        self.assertEqual(self.events[1]["pixel_height"], 480)
        self.assertEqual(self.events[1]["backing_scale"], 2.0)
        self.assertEqual(self.events[2]["backing_scale"], 2.0)
        driver.validate_normalized_events(self.events[:3], self.expected[:3])
        malformed = [dict(event) for event in self.events[:3]]
        malformed[1]["pixel_width"] = 0
        with self.assertRaises(driver.DriverError):
            driver.validate_normalized_events(malformed, self.expected[:3])

    def test_key_and_pointer_normalization(self) -> None:
        selection = self.events[3:8]
        self.assertEqual((self.events[3]["key_code"], self.events[3]["text"]), (0, "a"))
        self.assertEqual((self.events[5]["x"], self.events[5]["y"]), (12, 18))
        self.assertEqual((self.events[6]["button"], self.events[7]["button"]), (0, 0))
        driver.validate_normalized_events(selection, self.expected[3:8])
        malformed = [dict(event) for event in selection]
        malformed[-1]["button"] = -1
        with self.assertRaises(driver.DriverError):
            driver.validate_normalized_events(malformed, self.expected[3:8])

    def test_clean_quit(self) -> None:
        driver.validate_normalized_events([self.events[-1]], ["close_request"])

    def test_missing_or_malformed_events_rejected(self) -> None:
        with self.assertRaises(driver.DriverError):
            driver.validate_normalized_events(self.events[:-1], self.expected)
        malformed = [dict(event) for event in self.events]
        malformed[2]["backing_scale"] = float("nan")
        with self.assertRaises(driver.DriverError):
            driver.validate_normalized_events(malformed, self.expected)
        with self.assertRaises(driver.DriverError):
            driver.validate_native_smoke_output("native smoke events: open resize\ncompleted cycles: 100\n", 100)

    def test_100_cycles(self) -> None:
        cache = driver.ROOT / ".cache"
        cache.mkdir(exist_ok=True)
        script = driver.ROOT / "app" / "test_driver" / "driver.py"
        with tempfile.TemporaryDirectory(prefix="platform-cycles-", dir=cache) as temporary:
            output_root = Path(temporary) / "native"
            result = subprocess.run([
                sys.executable, str(script), "--native-platform-smoke", "--cycles", "100",
                "--output-dir", str(output_root),
            ], text=True, capture_output=True, timeout=60, check=True)
            self.assertIn("completed cycles: 100", result.stdout)
            report = json.loads((output_root / "run-0001" / "result.json").read_text())
            self.assertEqual(report["cycles"], 100)
            self.assertEqual(report["status"], "passed")

    def test_native_output_root_is_repeatable_without_overwrite(self) -> None:
        cache = driver.ROOT / ".cache"
        cache.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="platform-driver-", dir=cache) as temporary:
            root = Path(temporary) / "native"
            first = driver._safe_native_output(str(root))
            second = driver._safe_native_output(str(root))
            self.assertEqual(first.name, "run-0001")
            self.assertEqual(second.name, "run-0002")
            self.assertTrue(first.is_dir())
            self.assertTrue(second.is_dir())
            alias = Path(temporary) / "alias"
            alias.symlink_to(root, target_is_directory=True)
            with self.assertRaises(driver.DriverError):
                driver._safe_native_output(str(alias))

    def test_scalar_tile_values_bounds_and_overflow(self) -> None:
        binary = driver.ROOT / "build" / "mac-debug" / "bin" / "svgworkbench"
        result = subprocess.run([str(binary), "--tile-cpu-smoke"], cwd=driver.ROOT,
                                text=True, capture_output=True, timeout=10, check=True)
        self.assertEqual(result.stdout.splitlines(), [
            "tile bytes: 48", "first rgba: 0,0,0,255", "last rgba: 57,73,47,255"])
        first = bytes((0, 0, 0, 255))
        last = bytes(((3 * 17 + 2 * 3) & 255, (3 * 5 + 2 * 29) & 255,
                      (3 * 11 + 2 * 7) & 255, 255))
        self.assertEqual(last, bytes((57, 73, 47, 255)))
        self.assertEqual(len(first + last), 8)

    def test_native_metal_exact_readback_and_resized_frame(self) -> None:
        cache = driver.ROOT / ".cache"
        cache.mkdir(exist_ok=True)
        with tempfile.TemporaryDirectory(prefix="metal-smoke-", dir=cache) as temporary:
            out = driver._safe_native_output(str(Path(temporary) / "native"))
            elapsed = driver.native_metal_smoke(out)
            report = json.loads((out / "result.json").read_text())
            self.assertGreaterEqual(elapsed, 0.0)
            self.assertEqual(report["status"], "passed")
            self.assertEqual(report["frames"], 2)
            self.assertEqual(report["readback"], "byte-exact")
            self.assertEqual(report["resize_reset"], "passed")


if __name__ == "__main__":
    unittest.main()
