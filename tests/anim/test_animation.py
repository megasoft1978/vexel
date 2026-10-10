"""Independent timing/interpolation checks for the bounded SMIL profile."""

from __future__ import annotations

import json
import math
from pathlib import Path
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[2]
FIXTURES = ROOT / "tests" / "anim" / "fixtures"
CLI = ROOT / "build" / "mac-debug" / "bin" / "animation_eval_cli"
WORKBENCH = ROOT / "build" / "mac-debug" / "bin" / "svgworkbench"


def cubic(t: float, first: float, second: float) -> float:
    inverse = 1.0 - t
    return 3 * inverse * inverse * t * first + 3 * inverse * t * t * second + t**3


def spline(progress: float, points: tuple[float, float, float, float]) -> float:
    low, high = 0.0, 1.0
    for _ in range(60):
        parameter = (low + high) / 2
        if cubic(parameter, points[0], points[2]) < progress:
            low = parameter
        else:
            high = parameter
    return cubic((low + high) / 2, points[1], points[3])


def reference_value(
    seconds: float,
    values: list[float],
    duration: float,
    key_times: list[float],
    calc_mode: str = "linear",
    spline_points: tuple[float, float, float, float] | None = None,
    repeat_count: float = 1.0,
    freeze: bool = False,
) -> float | None:
    if seconds < 0 or duration <= 0:
        raise ValueError("invalid reference time")
    elapsed = seconds
    active_duration = duration * repeat_count
    if elapsed >= active_duration:
        if not freeze:
            return None
        remainder = active_duration % duration
        progress = 1.0 if remainder == 0 else remainder / duration
    else:
        progress = (elapsed % duration) / duration

    right = 1
    while right + 1 < len(key_times) and progress >= key_times[right]:
        right += 1
    left = right - 1
    local = (progress - key_times[left]) / (key_times[right] - key_times[left])
    if calc_mode == "discrete":
        return values[left]
    if calc_mode == "spline":
        if spline_points is None:
            raise ValueError("spline controls are required")
        local = spline(local, spline_points)
    return values[left] + (values[right] - values[left]) * local


def cli_tracks(fixture: str, seconds: float) -> dict:
    result = subprocess.run(
        [str(CLI), str(FIXTURES / fixture), f"{seconds:.17g}"],
        check=True,
        capture_output=True,
        text=True,
    )
    return json.loads(result.stdout)


class AnimationReferenceTests(unittest.TestCase):
    def test_authored_fixture_inventory(self) -> None:
        fixtures = sorted(FIXTURES.glob("*.svg"))
        self.assertGreaterEqual(len(fixtures), 15)
        self.assertTrue(all(path.read_text(encoding="utf-8").startswith("<svg") for path in fixtures))

    def test_linear_opacity_matches_python_at_sampled_times(self) -> None:
        values = [1.0, 0.2, 1.0]
        key_times = [0.0, 0.5, 1.0]
        for sample in range(100):
            seconds = sample * 0.02
            expected = reference_value(seconds, values, 1.0, key_times,
                                       repeat_count=math.inf)
            actual = cli_tracks("loader-opacity.svg", seconds)["tracks"][0]
            self.assertTrue(actual["active"])
            self.assertAlmostEqual(actual["sample"]["value"], expected, places=10)

    def test_spline_and_discrete_match_python(self) -> None:
        spline_values = [0.0, 1.0]
        for sample in range(1, 50):
            seconds = sample / 50
            expected = reference_value(seconds, spline_values, 1.0, [0.0, 1.0],
                calc_mode="spline", spline_points=(0.42, 0.0, 0.58, 1.0))
            actual = cli_tracks("spline-opacity.svg", seconds)["tracks"][0]
            self.assertAlmostEqual(actual["sample"]["value"], expected, delta=1e-8)
        discrete = cli_tracks("discrete-opacity.svg", 0.75)["tracks"][0]
        self.assertEqual(discrete["sample"]["value"], 0.2)

    def test_syncbase_fixture_lists_both_tracks(self) -> None:
        result = cli_tracks("repeat-wave.svg", 5.0)
        self.assertEqual([item["attribute"] for item in result["tracks"]],
                         ["x", "opacity"])
        self.assertTrue(result["tracks"][0]["active"])
        self.assertTrue(result["tracks"][1]["active"])

    def test_render_frame_accepts_explicit_time(self) -> None:
        with tempfile.TemporaryDirectory(prefix="anim-cli-") as directory:
            output = Path(directory) / "frame.ppm"
            result = subprocess.run(
                [str(WORKBENCH), "--render-frame", str(FIXTURES / "slide-x.svg"),
                 "--size", "48x16", "--out", str(output), "--time", "1"],
                cwd=ROOT,
                capture_output=True,
                text=True,
                timeout=20,
                check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertTrue(output.is_file())
            self.assertTrue(output.read_bytes().startswith(b"P6\n48 16\n255\n"))

    def test_render_animation_writes_bounded_ppm_sequence(self) -> None:
        with tempfile.TemporaryDirectory(prefix="anim-sequence-") as directory:
            output = Path(directory) / "frames"
            output.mkdir()
            result = subprocess.run(
                [str(WORKBENCH), "--render-animation", str(FIXTURES / "slide-x.svg"),
                 "--fps", "2", "--duration", "1", "--out", str(output)],
                cwd=ROOT, capture_output=True, text=True, timeout=30, check=False,
            )
            self.assertEqual(result.returncode, 0, result.stderr)
            frames = sorted(output.glob("frame-*.ppm"))
            self.assertEqual(len(frames), 2)
            self.assertTrue(all(frame.read_bytes().startswith(b"P6\n48 16\n255\n")
                                for frame in frames))

    def test_render_animation_rejects_more_than_300_frames(self) -> None:
        with tempfile.TemporaryDirectory(prefix="anim-sequence-limit-") as directory:
            result = subprocess.run(
                [str(WORKBENCH), "--render-animation", str(FIXTURES / "slide-x.svg"),
                 "--fps", "60", "--duration", "6", "--out", directory],
                cwd=ROOT, capture_output=True, text=True, timeout=10, check=False,
            )
            self.assertEqual(result.returncode, 2)
            self.assertIn("at most 300 frames", result.stderr)


if __name__ == "__main__":
    unittest.main()
