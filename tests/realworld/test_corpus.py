"""Differential checks for the authored real-world SVG fixture corpus."""
import hashlib
import json
import shutil
import subprocess
import tempfile
import time
import unittest
from pathlib import Path

from tests.svg.shape.png_utils import png_rgba

ROOT = Path(__file__).resolve().parents[2]
TEMP_ROOT = ROOT / ".cache"
TEMP_ROOT.mkdir(exist_ok=True)
CLI = ROOT / "build/mac-debug/bin/shape_render_cli"
REALWORLD_MANIFEST = ROOT / "tests/realworld/manifest.json"
REALWORLD_OUT = TEMP_ROOT / "realworld-decode2-results.json"
REALWORLD_MAX_PIXEL_ERROR = 160
REALWORLD_MAX_JOINT_RGB_ERROR = 128
REALWORLD_PROCESS_TIMEOUT = 8.0


class RealWorldCorpusTests(unittest.TestCase):
    def test_realworld_svg_corpus_against_librsvg(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by the real-world corpus")
        self.assertTrue(CLI.is_file(), "run ./dev build before the real-world corpus")
        manifest = json.loads(REALWORLD_MANIFEST.read_text(encoding="utf-8"))
        self.assertEqual(manifest["count"], 87)
        records = []
        failures = []
        with tempfile.TemporaryDirectory(prefix="vexel-realworld-",
                                         dir=TEMP_ROOT) as temp:
            temp_path = Path(temp)
            for case in manifest["cases"]:
                source_path = ROOT / "tests/realworld" / case["file"]
                source = source_path.read_bytes()
                digest = hashlib.sha256(source).hexdigest()
                if digest != case["sha256"]:
                    failures.append(f"{case['file']}: source hash differs from manifest")
                    continue
                ours_path = temp_path / f"{source_path.stem}.rgba"
                reference_path = temp_path / f"{source_path.stem}.png"
                started = time.perf_counter()
                try:
                    ours = subprocess.run([str(CLI), str(source_path), str(ours_path)],
                                          capture_output=True, text=True,
                                          timeout=REALWORLD_PROCESS_TIMEOUT)
                except subprocess.TimeoutExpired:
                    failures.append(f"{case['file']}: Vexel exceeded {REALWORLD_PROCESS_TIMEOUT}s")
                    continue
                vexel_seconds = time.perf_counter() - started
                started = time.perf_counter()
                try:
                    reference = subprocess.run(
                        [renderer, "--format", "png", "--output", str(reference_path),
                         str(source_path)], capture_output=True, text=True,
                        timeout=REALWORLD_PROCESS_TIMEOUT)
                except subprocess.TimeoutExpired:
                    failures.append(f"{case['file']}: rsvg-convert exceeded "
                                    f"{REALWORLD_PROCESS_TIMEOUT}s")
                    continue
                rsvg_seconds = time.perf_counter() - started
                record = {"file": case["file"], "category": case["category"],
                          "source_sha256": digest, "vexel_seconds": vexel_seconds,
                          "rsvg_seconds": rsvg_seconds, "supported": case["supported"]}
                if vexel_seconds > REALWORLD_PROCESS_TIMEOUT:
                    failures.append(f"{case['file']}: Vexel exceeded its wall-time bound")
                if rsvg_seconds > REALWORLD_PROCESS_TIMEOUT:
                    failures.append(f"{case['file']}: rsvg-convert exceeded its wall-time bound")
                if reference.returncode != 0:
                    failures.append(f"{case['file']}: rsvg-convert failed: {reference.stderr.strip()}")
                    records.append(record)
                    continue
                if case["supported"]:
                    if ours.returncode != 0:
                        failures.append(f"{case['file']}: supported SVG returned "
                                        f"{ours.stderr.strip()}")
                        record["diagnostic"] = ours.stderr.strip()
                        records.append(record)
                        continue
                else:
                    expected = case["diagnostic"]
                    category = ours.stderr.split(":", 1)[0] if ours.stderr else ""
                    if ours.returncode == 0 or category != expected:
                        failures.append(f"{case['file']}: expected stable {expected} diagnostic, "
                                        f"got rc={ours.returncode} {ours.stderr.strip()}")
                    record["diagnostic"] = ours.stderr.strip()
                    records.append(record)
                    continue
                width, height, expected = png_rgba(reference_path.read_bytes())
                actual = ours_path.read_bytes()
                if (width, height) != (case["width"], case["height"]):
                    failures.append(f"{case['file']}: rsvg dimensions {(width, height)} differ "
                                    f"from manifest {(case['width'], case['height'])}")
                if len(actual) != width * height * 4:
                    failures.append(f"{case['file']}: Vexel output has {len(actual)} bytes, "
                                    f"expected {width * height * 4}")
                    records.append(record)
                    continue
                max_pixel_error = 0
                max_joint_rgb_error = 0
                differing_pixels = 0
                for offset in range(0, len(actual), 4):
                    alpha_delta = abs(actual[offset + 3] - expected[offset + 3])
                    delta = alpha_delta
                    if actual[offset + 3] != 0 and expected[offset + 3] != 0:
                        rgb_delta = max(abs(actual[offset + channel] - expected[offset + channel])
                                        for channel in range(3))
                        max_joint_rgb_error = max(max_joint_rgb_error, rgb_delta)
                        delta = max(delta, rgb_delta)
                    if delta != 0:
                        differing_pixels += 1
                        max_pixel_error = max(max_pixel_error, delta)
                record.update({"width": width, "height": height,
                               "max_pixel_error": max_pixel_error,
                               "max_joint_rgb_error": max_joint_rgb_error,
                               "differing_pixels": differing_pixels,
                               "status": "pass"})
                if max_pixel_error > REALWORLD_MAX_PIXEL_ERROR:
                    failures.append(f"{case['file']}: max pixel error {max_pixel_error} "
                                    f"exceeds {REALWORLD_MAX_PIXEL_ERROR}")
                    record["status"] = "fail"
                if max_joint_rgb_error > REALWORLD_MAX_JOINT_RGB_ERROR:
                    failures.append(f"{case['file']}: jointly-covered RGB error "
                                    f"{max_joint_rgb_error} exceeds "
                                    f"{REALWORLD_MAX_JOINT_RGB_ERROR}")
                    record["status"] = "fail"
                if vexel_seconds >= REALWORLD_PROCESS_TIMEOUT or \
                        rsvg_seconds >= REALWORLD_PROCESS_TIMEOUT:
                    record["status"] = "fail"
                records.append(record)
                self.assertEqual(digest, hashlib.sha256(source_path.read_bytes()).hexdigest(),
                                 case["file"])
        REALWORLD_OUT.write_text(json.dumps({
            "count": len(manifest["cases"]), "renderer": str(CLI),
            "reference_renderer": renderer, "process_timeout_seconds": REALWORLD_PROCESS_TIMEOUT,
            "max_pixel_error_threshold": REALWORLD_MAX_PIXEL_ERROR,
            "max_joint_rgb_error_threshold": REALWORLD_MAX_JOINT_RGB_ERROR,
            "cases": records, "failures": failures,
            "status": "pass" if not failures else "fail",
        }, indent=2) + "\n", encoding="utf-8")
        self.assertEqual(len(records), 87, "every manifest fixture must have a result")
        self.assertEqual(failures, [], "real-world results: " + str(REALWORLD_OUT))


