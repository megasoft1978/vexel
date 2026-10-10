from __future__ import annotations

import json
import os
import pathlib
import subprocess
import tempfile
import unittest


ROOT = pathlib.Path(__file__).resolve().parents[1]
PROBE = pathlib.Path(
    os.environ.get("VEXEL_MEDIA_PROBE", ROOT / "build/mac-debug/bin/media_probe")
)
GENERATOR = ROOT / "tests/platform_media/generate_fixtures.py"


class PlatformMediaCliTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        if not PROBE.is_file():
            raise RuntimeError(f"media_probe is missing; build it first: {PROBE}")
        cls.temporary = tempfile.TemporaryDirectory(prefix="vexel-platform-media-")
        cls.fixture_dir = pathlib.Path(cls.temporary.name) / "fixtures"
        subprocess.run(
            ["python3", str(GENERATOR), "--output-dir", str(cls.fixture_dir)],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

    @classmethod
    def tearDownClass(cls) -> None:
        cls.temporary.cleanup()

    def run_probe(self, *arguments: str) -> subprocess.CompletedProcess[bytes]:
        return subprocess.run(
            [str(PROBE), *arguments],
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            check=False,
        )

    def test_text_probe_emits_structural_outline_json(self) -> None:
        result = self.run_probe("text", "Vexel", "sans-serif")
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
        data = json.loads(result.stdout)
        self.assertEqual(data["glyph_count"], 5)
        self.assertGreater(data["advance_width"], 0.0)
        self.assertGreater(data["ascent"], 0.0)
        self.assertGreater(data["command_count"], 0)
        self.assertEqual(sum(len(glyph["commands"]) for glyph in data["glyphs"]),
                         data["command_count"])
        self.assertTrue(all(command["op"] in {"M", "L", "C", "Z"}
                            for glyph in data["glyphs"]
                            for command in glyph["commands"]))

    def test_image_probe_writes_top_down_ppm_with_expected_size(self) -> None:
        ppm_path = pathlib.Path(self.temporary.name) / "sample.ppm"
        result = self.run_probe("image", str(self.fixture_dir / "sample.png"),
                                str(ppm_path))
        self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", "replace"))
        ppm = ppm_path.read_bytes()
        header, pixels = ppm.split(b"\n255\n", 1)
        self.assertEqual(header, b"P6\n3 2")
        self.assertEqual(len(pixels), 3 * 2 * 3)
        self.assertEqual(pixels[:3], bytes((255, 0, 0)))
        self.assertEqual(pixels[3:6], bytes((127, 255, 127)))

    def test_jpeg_and_static_gif_probe(self) -> None:
        for filename in ("sample.jpg", "static.gif"):
            ppm_path = pathlib.Path(self.temporary.name) / f"{filename}.ppm"
            result = self.run_probe("image", str(self.fixture_dir / filename),
                                    str(ppm_path))
            self.assertEqual(result.returncode, 0,
                             result.stderr.decode("utf-8", "replace"))
            self.assertTrue(ppm_path.read_bytes().startswith(b"P6\n"))

    def test_animation_and_oversized_dimensions_have_stable_statuses(self) -> None:
        animated = self.run_probe("image", str(self.fixture_dir / "animated.gif"),
                                  str(pathlib.Path(self.temporary.name) / "unused.ppm"))
        self.assertNotEqual(animated.returncode, 0)
        self.assertEqual(animated.stderr.strip(), b"animated")
        oversized = self.run_probe(
            "image", str(self.fixture_dir / "oversized-dimension.png"),
            str(pathlib.Path(self.temporary.name) / "unused2.ppm"),
        )
        self.assertNotEqual(oversized.returncode, 0)
        self.assertEqual(oversized.stderr.strip(), b"limit")

    def test_emoji_has_a_stable_unsupported_status(self) -> None:
        result = self.run_probe("text", "😀")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(result.stderr.strip(), b"unsupported")


if __name__ == "__main__":
    unittest.main()
