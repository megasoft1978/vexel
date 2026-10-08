"""Focused acceptance coverage for the native platform driver contract."""

from __future__ import annotations

import sys
import tempfile
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
        for _ in range(100):
            driver.validate_normalized_events(self.events, self.expected)
        self.assertEqual(driver.validate_native_smoke_output(
            "native smoke events: open resize scale key pointer close\ncompleted cycles: 100\n", 100)[1],
            "completed cycles: 100")

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


if __name__ == "__main__":
    unittest.main()
