"""Contract tests for the deterministic placeholder UI script driver."""

from __future__ import annotations

import json
import struct
import subprocess
import sys
import tempfile
import unittest
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DRIVER = ROOT / "app/test_driver/driver.py"
SAMPLE = ROOT / "tests/ui/mixed-commands.jsonl"
CACHE = ROOT / ".cache"


class DriverTests(unittest.TestCase):
    def setUp(self) -> None:
        CACHE.mkdir(exist_ok=True)
        self.temp = tempfile.TemporaryDirectory(prefix="ui-driver-", dir=CACHE)
        self.work = Path(self.temp.name)

    def tearDown(self) -> None:
        self.temp.cleanup()

    def script(self, name: str, commands: list[object] | str) -> Path:
        path = self.work / name
        if isinstance(commands, str):
            path.write_text(commands, encoding="utf-8")
        else:
            path.write_text("".join(json.dumps(item, separators=(",", ":")) + "\n" for item in commands), encoding="utf-8")
        return path

    def output_path(self, label: str) -> Path:
        return Path(".cache") / self.work.name / label

    def run_driver(self, script: Path, output: Path, *extra: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run([sys.executable, str(DRIVER), "--script", str(script),
                               "--output-dir", str(output), *extra], cwd=ROOT,
                              text=True, capture_output=True, check=False)

    @staticmethod
    def png_dimensions(path: Path) -> tuple[int, int]:
        data = path.read_bytes()
        if not data.startswith(b"\x89PNG\r\n\x1a\n"):
            raise AssertionError("missing PNG signature")
        offset = 8
        seen_ihdr = seen_iend = False
        while offset < len(data):
            length = struct.unpack_from(">I", data, offset)[0]
            kind = data[offset + 4:offset + 8]
            payload = data[offset + 8:offset + 8 + length]
            crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
            if zlib.crc32(kind + payload) & 0xFFFFFFFF != crc:
                raise AssertionError("bad PNG chunk CRC")
            if kind == b"IHDR":
                seen_ihdr = True
                width, height, depth, color, comp, filt, interlace = struct.unpack(">2I5B", payload)
                if (depth, color, comp, filt, interlace) != (8, 6, 0, 0, 0):
                    raise AssertionError("wrong PNG profile")
            if kind == b"IEND":
                seen_iend = True
            offset += 12 + length
        if not seen_ihdr or not seen_iend:
            raise AssertionError("incomplete PNG")
        return width, height

    def test_all_commands_emit_placeholder_artifacts(self) -> None:
        result = self.run_driver(SAMPLE, self.output_path("all"), "--seed", "17")
        self.assertEqual(result.returncode, 0, result.stderr)
        out = ROOT / self.output_path("all")
        self.assertEqual(self.png_dimensions(out / "frame-0000.png"), (640, 360))
        self.assertEqual(self.png_dimensions(out / "frame-0001.png"), (640, 360))
        tree = json.loads((out / "accessibility-0000.json").read_text())
        self.assertEqual(tree["role"], "application")
        self.assertEqual([node["id"] for node in tree["children"]], ["canvas", "progress"])
        saved = json.loads((out / "document.json").read_text())
        self.assertEqual(saved["document_id"], "smoke-document")
        self.assertEqual(saved["virtual_time_ns"], 16_666_667)
        acknowledgments = [json.loads(line) for line in result.stdout.splitlines()]
        self.assertEqual(len(acknowledgments), 15)
        self.assertEqual(acknowledgments[-2]["outputs"], ["frame-0001.png"])

    def test_identical_replay_is_byte_identical(self) -> None:
        one, two = self.output_path("replay-one"), self.output_path("replay-two")
        a = self.run_driver(SAMPLE, one, "--seed", "17")
        b = self.run_driver(SAMPLE, two, "--seed", "17")
        self.assertEqual((a.returncode, a.stdout, a.stderr), (b.returncode, b.stdout, b.stderr))
        first, second = ROOT / one, ROOT / two
        names_a = sorted(p.name for p in first.iterdir())
        names_b = sorted(p.name for p in second.iterdir())
        self.assertEqual(names_a, names_b)
        self.assertEqual({name: (first / name).read_bytes() for name in names_a},
                         {name: (second / name).read_bytes() for name in names_b})
        seed_out = self.output_path("different-seed")
        c = self.run_driver(SAMPLE, seed_out, "--seed", "18")
        self.assertEqual(c.returncode, 0, c.stderr)
        self.assertNotEqual((first / "frame-0000.png").read_bytes(),
                            (ROOT / seed_out / "frame-0000.png").read_bytes())

    def test_invalid_cli_and_jsonl_are_rejected(self) -> None:
        missing = subprocess.run([sys.executable, str(DRIVER)], cwd=ROOT, text=True, capture_output=True)
        self.assertEqual(missing.returncode, 2)
        bad_seed = self.run_driver(SAMPLE, self.output_path("bad-seed"), "--seed", "4294967296")
        self.assertEqual(bad_seed.returncode, 2)
        valid = [{"command": "open", "document_id": "d"}, {"command": "quit"}]
        invalid = [
            "\n".join(json.dumps(item) for item in valid) + "\n\n",
            '{"command":"open","document_id":"d","document_id":"e"}\n{"command":"quit"}\n',
            '{"command":"open","document_id":"d","extra":1}\n{"command":"quit"}\n',
            "[]\n{\"command\":\"quit\"}\n",
            '{"command":"open","document_id":"d"}\n{"command":"resize","width":true,"height":4}\n{"command":"quit"}\n',
            '{"command":"open","document_id":"d"}\n{"command":"frame","delta_ns":NaN}\n{"command":"quit"}\n',
            '{"command":"open","document_id":"d"}\n{"command":"unknown"}\n{"command":"quit"}\n',
            '{"command":"open","document_id":"d"}\n{"command":"quit"}\n{"command":"save"}\n',
            '{"command":"open","document_id":"d"}\n',
            '{"command":"open","document_id":"d"}\n{"command":"pointer","action":[],"x":0,"y":0}\n{"command":"quit"}\n',
            '{"command":"open","document_id":"d"}\n{"command":"key","action":{},"key":"x"}\n{"command":"quit"}\n',
        ]
        for index, content in enumerate(invalid):
            with self.subTest(index=index):
                script = self.script(f"invalid-{index}.jsonl", content)
                output = self.output_path(f"invalid-{index}")
                result = self.run_driver(script, output)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertEqual(result.stdout, "")
                self.assertFalse((ROOT / output).exists())

    def test_path_escape_symlink_and_existing_output_are_rejected(self) -> None:
        script = self.script("valid.jsonl", [{"command": "open", "document_id": "d"}, {"command": "quit"}])
        escaped = self.run_driver(script, Path(".cache") / ".." / "outside")
        self.assertEqual(escaped.returncode, 2)
        existing = self.work / "already"
        existing.mkdir()
        result = self.run_driver(script, existing)
        self.assertEqual(result.returncode, 2)
        target, link = self.work / "target", self.work / "link"
        target.mkdir()
        link.symlink_to(target, target_is_directory=True)
        result = self.run_driver(script, link / "new-output")
        self.assertEqual(result.returncode, 2)
        outside = ROOT.parent / "vexel-outside-script.jsonl"
        result = self.run_driver(outside, self.output_path("outside-script"))
        self.assertEqual(result.returncode, 2)
        self.assertIn("script path must stay inside", result.stderr)

    def test_bounds_and_command_order_are_enforced(self) -> None:
        head, tail = {"command": "open", "document_id": "d"}, {"command": "quit"}
        cases = [
            [head, {"command": "resize", "width": 4097, "height": 1}, tail],
            [head, {"command": "pointer", "action": "move", "x": 320, "y": 0}, tail],
            [head, {"command": "frame", "delta_ns": 1_000_000_001}, tail],
            [head] + [{"command": "frame", "delta_ns": 1_000_000_000}] * 1001 + [tail],
            [head, {"command": "open", "document_id": "second"}, tail],
            [tail], [head, {"command": "quit"}, tail],
            [head, {"command": "progress", "percent": 101, "label": "x"}, tail],
            [head, {"command": "key", "action": "press", "key": "x"}, tail],
        ]
        for index, commands in enumerate(cases):
            with self.subTest(index=index):
                script = self.script(f"bounds-{index}.jsonl", commands)
                result = self.run_driver(script, self.output_path(f"bounds-{index}"))
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertEqual(result.stdout, "")
        oversized = self.work / "large.jsonl"
        oversized.write_bytes((b" " * 65_537) + b"\n")
        result = self.run_driver(oversized, self.output_path("large"))
        self.assertEqual(result.returncode, 2)
        too_many = self.work / "too-many.jsonl"
        too_many.write_bytes((b'{"command":"open","document_id":"d"}\n'
                              + b'{"command":"frame","delta_ns":0}\n' * 9_999
                              + b'{"command":"quit"}\n'))
        result = self.run_driver(too_many, self.output_path("too-many"))
        self.assertEqual(result.returncode, 2)
        too_large = self.work / "too-large.jsonl"
        too_large.write_bytes((b'{"command":"open","document_id":"'
                               + b"x" * (8 * 1024 * 1024) + b'"}\n'))
        result = self.run_driver(too_large, self.output_path("too-large"))
        self.assertEqual(result.returncode, 2)
        maximum = self.run_driver(self.script("maximum.jsonl", [head,
            {"command": "resize", "width": 4096, "height": 4096}, tail]), self.output_path("maximum"))
        self.assertEqual(maximum.returncode, 0, maximum.stderr)

    def test_smoke_output_contract(self) -> None:
        output = self.output_path("smoke-contract")
        result = self.run_driver(SAMPLE, output, "--seed", "17")
        self.assertEqual(result.returncode, 0, result.stderr)
        path = ROOT / output
        self.assertTrue(list(path.glob("frame-*.png")))
        self.assertTrue(list(path.glob("accessibility-*.json")))
        self.assertTrue((path / "document.json").is_file())


if __name__ == "__main__":
    unittest.main()
