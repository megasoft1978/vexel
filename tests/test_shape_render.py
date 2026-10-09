"""Differential and deterministic integration checks for the bounded shape subset."""
import hashlib
import json
import platform
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FIXTURES = ROOT / "tests/svg/shape/fixtures"
CLI = ROOT / "build/mac-debug/bin/shape_render_cli"
OUT = ROOT / "artifacts/WO-M4-04/reference-comparison.json"


def png_rgba(data):
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("not PNG")
    pos, compressed, width, height, bpp = 8, bytearray(), 0, 0, 0
    while pos < len(data):
        size = struct.unpack(">I", data[pos:pos + 4])[0]
        kind = data[pos + 4:pos + 8]
        payload = data[pos + 8:pos + 8 + size]
        if kind == b"IHDR":
            width, height, depth, color, comp, filt, interlace = struct.unpack(">IIBBBBB", payload)
            if depth != 8 or color != 6 or interlace != 0:
                raise ValueError("reference PNG must be noninterlaced RGBA8")
            bpp = 4
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
        pos += size + 12
    raw = zlib.decompress(compressed)
    stride = width * bpp
    rows, previous, off = [], bytearray(stride), 0
    for _ in range(height):
        f = raw[off]
        row = bytearray(raw[off + 1:off + 1 + stride])
        off += stride + 1
        for i in range(stride):
            a = row[i - bpp] if i >= bpp else 0
            b = previous[i]
            c = previous[i - bpp] if i >= bpp else 0
            if f == 1:
                row[i] = (row[i] + a) & 255
            elif f == 2:
                row[i] = (row[i] + b) & 255
            elif f == 3:
                row[i] = (row[i] + ((a + b) // 2)) & 255
            elif f == 4:
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                predictor = a if pa <= pb and pa <= pc else b if pb <= pc else c
                row[i] = (row[i] + predictor) & 255
            elif f != 0:
                raise ValueError("unknown PNG filter")
        rows.append(row)
        previous = row
    return width, height, b"".join(rows)


class ShapeRenderTests(unittest.TestCase):
    def test_decoder_diagnostics_source_retention_and_raster(self):
        self.assertTrue(CLI.is_file(), "run ./dev build before the focused shape tests")
        result = subprocess.run([str(ROOT / "build/mac-debug/bin/shape_test")], check=True, capture_output=True, text=True)
        self.assertIn("shape decoder and 4x4 antialiased raster checks passed", result.stdout)

    def test_reference_comparison_and_repeatability(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by this work order")
        version = subprocess.run([renderer, "--version"], check=True, capture_output=True, text=True).stdout.strip()
        results = []
        with tempfile.TemporaryDirectory(prefix="vexel-shape-") as temp:
            temp_path = Path(temp)
            for fixture in sorted(FIXTURES.glob("*.svg")):
                raw = fixture.read_bytes()
                first, second, reference = (temp_path / f"{fixture.stem}-{i}.rgba" for i in range(3))
                subprocess.run([str(CLI), str(fixture), str(first)], check=True, capture_output=True)
                subprocess.run([str(CLI), str(fixture), str(second)], check=True, capture_output=True)
                self.assertEqual(first.read_bytes(), second.read_bytes(), fixture.name)
                subprocess.run([renderer, "--format", "png", "--output", str(reference), str(fixture)], check=True, capture_output=True)
                rw, rh, reference_rgba = png_rgba(reference.read_bytes())
                ours = first.read_bytes()
                width, height = 16, 16
                self.assertEqual((rw, rh), (width, height))
                self.assertEqual(len(ours), width * height * 4)
                # Renderer antialiasing may affect pixels within one pixel of a
                # fill-coverage boundary; every interior channel must match exactly.
                bad_interior, edge_diffs, max_channel = 0, 0, 0
                max_alpha_delta, max_rgb_delta = 0, 0
                for y in range(height):
                    for x in range(width):
                        offset = (y * width + x) * 4
                        delta = max(abs(ours[offset + c] - reference_rgba[offset + c]) for c in range(4))
                        max_alpha_delta = max(max_alpha_delta, abs(ours[offset + 3] - reference_rgba[offset + 3]))
                        if ours[offset + 3] != 0 and reference_rgba[offset + 3] != 0:
                            for channel in range(3):
                                max_rgb_delta = max(
                                    max_rgb_delta,
                                    abs(ours[offset + channel] - reference_rgba[offset + channel]))
                        if not delta:
                            continue
                        max_channel = max(max_channel, delta)
                        alpha_boundary = False
                        ours_color_boundary = False
                        reference_color_boundary = False
                        for yy in range(max(0, y - 1), min(height, y + 2)):
                            for xx in range(max(0, x - 1), min(width, x + 2)):
                                oi = (yy * width + xx) * 4
                                alpha_boundary |= (ours[oi + 3] != ours[offset + 3] or
                                                   reference_rgba[oi + 3] != reference_rgba[offset + 3])
                                ours_color_boundary |= ours[oi:oi + 3] != ours[offset:offset + 3]
                                reference_color_boundary |= (
                                    reference_rgba[oi:oi + 3] != reference_rgba[offset:offset + 3])
                        edge = alpha_boundary or (ours_color_boundary and reference_color_boundary)
                        if edge:
                            edge_diffs += 1
                        else:
                            bad_interior += 1
                self.assertEqual(bad_interior, 0, fixture.name)
                self.assertLessEqual(edge_diffs, 120, fixture.name)
                self.assertLessEqual(max_alpha_delta, 160, fixture.name)
                self.assertLessEqual(max_rgb_delta, 1, fixture.name)
                results.append({"case": fixture.stem, "source_sha256": hashlib.sha256(raw).hexdigest(),
                                "vexel_sha256": hashlib.sha256(ours).hexdigest(),
                                "reference_png_sha256": hashlib.sha256(reference.read_bytes()).hexdigest(),
                                "edge_diff_pixels": edge_diffs, "interior_diff_pixels": bad_interior,
                                "max_channel_diff": max_channel,
                                "max_jointly_covered_straight_rgb_diff": max_rgb_delta,
                                "max_alpha_diff": max_alpha_delta})
        OUT.parent.mkdir(parents=True, exist_ok=True)
        host = {"platform": platform.platform(), "machine": platform.machine(),
                "macos_version": platform.mac_ver()[0] or "unavailable"}
        compiler = subprocess.run(["clang", "--version"], check=True, capture_output=True,
                                  text=True).stdout.splitlines()[0]
        OUT.write_text(json.dumps({"renderer_path": renderer, "renderer_version": version,
                "reference_renderer": "librsvg via rsvg-convert",
                "vexel_renderer": "Vexel scalar 4x4 shape rasterizer",
                "host": host, "compiler": compiler,
                "metric_availability": {"latency": "not collected by the differential fixture harness",
                                        "peak_rss": "not collected by the differential fixture harness"},
                "edge_tolerance": "zero interior differences; within one pixel of a fill-coverage boundary, alpha delta <=160; raw straight RGB delta <=1 for every jointly covered pixel; and at most 120 differing pixels per fixture",
                                   "cases": results}, indent=2) + "\n")

    def test_bounds_and_stable_diagnostics(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        cases = {
            "huge": '<svg width="16" height="16"><path d="M0 0 L1000001 1"/></svg>',
            "nonfinite": '<svg width="16" height="16"><path d="M0 0 Lnan 1"/></svg>',
            "malformed": '<svg width="16" height="16"><path d="M,,0 1"/></svg>',
            "missing_numeric_separator": '<svg width="16" height="16"><path d="M0 0 L1.2.3 4 5"/></svg>',
            "invalid_arc_flag": '<svg width="16" height="16"><path d="M0 0 A1 1 0 2 0 4 4"/></svg>',
            "nonliteral_arc_flag": '<svg width="16" height="16"><path d="M0 0 A1 1 0 1.0 0 4 4"/></svg>',
            "missing_initial_move": '<svg width="16" height="16"><path d="L0 0"/></svg>',
            "relative_curve_overflow": '<svg width="16" height="16"><path d="M999999 0 c2 2 2 2 2 2"/></svg>',
            "relative_quad_overflow": '<svg width="16" height="16"><path d="M999999 0 q2 2 2 2"/></svg>',
            "prefixed_svg_attribute": '<svg width="16" height="16"><rect x="1" y="1" width="4" height="4" x:fill="red"/></svg>',
            "wrong_default_namespace": '<svg xmlns="urn:other" width="16" height="16"/>',
            "trailing_root_dimension_comma": '<svg width="16," height="16"/>',
            "trailing_rect_dimension_comma": '<svg width="16" height="16"><rect x="1" y="1" width="4," height="4"/></svg>',
            "repeated_viewbox_commas": '<svg width="16" height="16" viewBox="0,,0,16,16"/>',
            "rect_missing_width": '<svg width="16" height="16"><rect height="4"/></svg>',
            "rect_missing_height": '<svg width="16" height="16"><rect width="4"/></svg>',
            "negative_circle_radius": '<svg width="16" height="16"><circle r="-1"/></svg>',
            "negative_ellipse_radius": '<svg width="16" height="16"><ellipse rx="1" ry="-1"/></svg>',
            "negative_rect_dimension": '<svg width="16" height="16"><rect width="-1" height="2"/></svg>',
            "odd_polygon_points": '<svg width="16" height="16"><polygon points="1 2 3"/></svg>',
            "trailing_polygon_comma": '<svg width="16" height="16"><polygon points="1 1 2 2,"/></svg>',
            "nan_transform": '<svg width="16" height="16"><g transform="translate(NaN 1)"/></svg>',
            "trailing_transform_comma": '<svg width="16" height="16"><g transform="translate(1,)"/></svg>',
            "skew_transform": '<svg width="16" height="16"><rect width="1" height="1" transform="skewX(10)"/></svg>',
            "transform_overflow": '<svg width="16" height="16"><rect width="1" height="1" transform="matrix(1e300 0 0 1 0 0)"/></svg>',
            "ellipse_segment_cap": '<svg width="16" height="16"><circle r="1000000"/></svg>',
            "negative_stroke_width": '<svg width="16" height="16"><line stroke="red" stroke-width="-1"/></svg>',
            "invalid_stroke_cap": '<svg width="16" height="16"><line stroke="red" stroke-linecap="triangle"/></svg>',
            "bad_dash_separator": '<svg width="16" height="16"><line stroke="red" stroke-dasharray="1,,2"/></svg>',
            "too_many_dash_values": '<svg width="16" height="16"><line stroke="red" stroke-dasharray="1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17"/></svg>',
            "unsupported_stroked_path": '<svg width="16" height="16"><path stroke="red" d="M1 1 L8 8"/></svg>',
            "unsupported_stylesheet_pi": '<?xml-stylesheet type="text/css" href="theme.css"?><svg width="16" height="16"/>',
        }
        stroke_diagnostic_offsets = {
            "negative_stroke_width": ("number", "-1"),
            "invalid_stroke_cap": ("unsupported", "triangle"),
            "bad_dash_separator": ("number", "1,,2"),
            "too_many_dash_values": ("number", "1 2 3 4 5 6 7 8 9 10 11 12 13 14 15 16 17"),
            "unsupported_stroked_path": ("unsupported", "red"),
        }
        report = {"cases": [], "source_retention": "input bytes hashed before and after invocation; CLI borrows file unchanged"}
        with tempfile.TemporaryDirectory(prefix="vexel-shape-bounds-") as temp:
            temp_path = Path(temp)
            for name, source in cases.items():
                src, out = temp_path / f"{name}.svg", temp_path / f"{name}.rgba"
                src.write_text(source)
                digest_before = hashlib.sha256(src.read_bytes()).hexdigest()
                run = subprocess.run([str(binary), str(src), str(out)], capture_output=True, text=True)
                digest_after = hashlib.sha256(src.read_bytes()).hexdigest()
                self.assertNotEqual(run.returncode, 0, name)
                category, offset = run.stderr.strip().split(":", 1)
                self.assertTrue(offset.isdigit(), name)
                self.assertEqual(digest_before, digest_after, name)
                if name in stroke_diagnostic_offsets:
                    expected_category, marker = stroke_diagnostic_offsets[name]
                    self.assertEqual(category, expected_category, name)
                    self.assertEqual(int(offset), source.index(marker), name)
                report["cases"].append({"name": name, "category": category, "offset": int(offset),
                                        "source_sha256": digest_before, "source_unchanged": True})
            # Curved groups exhaust the edge cap before the 65,536 command cap.
            path = "M0 0 " + " ".join("C1 15 2 -10 3 3" for _ in range(65535))
            src = temp_path / "edge-cap.svg"
            src.write_text(f'<svg width="16" height="16"><path d="{path}"/></svg>')
            run = subprocess.run([str(binary), str(src), str(temp_path / "edge-cap.rgba")], capture_output=True, text=True)
            self.assertNotEqual(run.returncode, 0)
            category, offset = run.stderr.strip().split(":", 1)
            self.assertEqual(category, "limit")
            report["cases"].append({"name": "flattened_edge_cap", "category": category, "offset": int(offset),
                                    "generated_curve_groups": 65535})
            source = '<svg width="16" height="16">' + '<g transform="translate(1)"/>' * 4097 + '</svg>'
            src = temp_path / "transform-count-cap.svg"
            src.write_text(source)
            run = subprocess.run([str(binary), str(src), str(temp_path / "transform-count-cap.rgba")], capture_output=True, text=True)
            self.assertNotEqual(run.returncode, 0)
            category, offset = run.stderr.strip().split(":", 1)
            self.assertEqual(category, "limit")
            report["cases"].append({"name": "transform_count_cap", "category": category, "offset": int(offset),
                                    "generated_transform_nodes": 4097})
            degenerate = temp_path / "degenerate.svg"
            degenerate.write_text('<svg width="16" height="16"><path d="M2 2 L2 2 Z"/></svg>')
            run = subprocess.run([str(binary), str(degenerate), str(temp_path / "degenerate.rgba")], capture_output=True)
            self.assertEqual(run.returncode, 0)
            report["cases"].append({"name": "degenerate_path", "result": "safe_empty_or_filled"})

    def test_independent_shape_fill_rules_and_painter_order(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = (b'<svg width="16" height="16">'
                  b'<rect x="1" y="1" width="11" height="11" fill="green"/>'
                  b'<rect x="5" y="5" width="10" height="10" fill="red" fill-rule="evenodd"/>'
                  b'<path fill="blue" fill-rule="evenodd" '
                  b'd="M2 2 H14 V14 H2 M5 5 H11 V11 H5"/>'
                  b'</svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-shape-order-") as temp:
            input_path, output_path = Path(temp) / "order.svg", Path(temp) / "order.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            rgba = output_path.read_bytes()
        pixel = lambda x, y: rgba[(y * 16 + x) * 4:(y * 16 + x) * 4 + 4]
        self.assertEqual(pixel(1, 1), bytes((0, 128, 0, 255)))
        self.assertEqual(pixel(7, 7), bytes((255, 0, 0, 255)))
        self.assertEqual(pixel(12, 12), bytes((0, 0, 255, 255)))
        self.assertEqual(pixel(8, 8), bytes((255, 0, 0, 255)))

    def test_fractional_viewport_rounds_tile_extent_up(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = b'<svg width="8.5px" height="7.25"/>'
        with tempfile.TemporaryDirectory(prefix="vexel-shape-fractional-") as temp:
            input_path, output_path = Path(temp) / "fractional.svg", Path(temp) / "fractional.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            self.assertEqual(output_path.stat().st_size, 9 * 8 * 4)

    def test_default_viewbox_meet_letterboxes_mismatched_aspect_ratio(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = (b'<svg width="16" height="16" viewBox="0 0 32 16">'
                  b'<rect x="0" y="0" width="32" height="16"/></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-viewbox-meet-") as temp:
            input_path, output_path = Path(temp) / "meet.svg", Path(temp) / "meet.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            rgba = output_path.read_bytes()
        alpha = lambda x, y: rgba[(y * 16 + x) * 4 + 3]
        self.assertEqual(alpha(8, 1), 0)
        self.assertEqual(alpha(8, 4), 255)
        self.assertEqual(alpha(8, 11), 255)
        self.assertEqual(alpha(8, 12), 0)

    def test_root_height_diagnostic_uses_height_value_offset(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        cases = ((b'<svg width="16" height="bad"/>', b'bad', "number"),
                 (b'<svg width="16" height="5000"/>', b'5000', "viewport"))
        with tempfile.TemporaryDirectory(prefix="vexel-height-offset-") as temp:
            for index, (source, marker, category_expected) in enumerate(cases):
                input_path = Path(temp) / f"height-{index}.svg"
                output_path = Path(temp) / f"height-{index}.rgba"
                input_path.write_bytes(source)
                run = subprocess.run([str(binary), str(input_path), str(output_path)],
                                     capture_output=True, text=True)
                category, offset = run.stderr.strip().split(":", 1)
                self.assertNotEqual(run.returncode, 0)
                self.assertEqual(category, category_expected)
                self.assertEqual(int(offset), source.index(marker))

    def test_transform_diagnostic_preserves_attribute_value_offset(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = b'<svg width="16" height="16"><g transform="skewX(10)"/></svg>'
        with tempfile.TemporaryDirectory(prefix="vexel-transform-offset-") as temp:
            input_path = Path(temp) / "offset.svg"
            output_path = Path(temp) / "offset.rgba"
            input_path.write_bytes(source)
            run = subprocess.run([str(binary), str(input_path), str(output_path)], capture_output=True, text=True)
        category, offset = run.stderr.strip().split(":", 1)
        self.assertNotEqual(run.returncode, 0)
        self.assertEqual(category, "unsupported")
        self.assertEqual(int(offset), source.index(b"skewX"))

    def test_shape_fuzz_wrapper_rejects_and_retains_sanitizer_stderr(self):
        script = ROOT / "tests/fuzz/shape/shape_fuzz.py"
        with tempfile.TemporaryDirectory(prefix="vexel-fuzz-stderr-") as temp:
            temp_path = Path(temp)
            binary = temp_path / "fake-shape-fuzz"
            binary.write_text("#!/usr/bin/env python3\nimport sys\n"
                              "print('seed=7 cases=1000000 duration=0.1 cap_exhaustion=1 cap_case_bytes=327730')\n"
                              "sys.stderr.write('runtime error: simulated UBSan diagnostic\\n')\n")
            binary.chmod(0o755)
            report = temp_path / "fuzz-report.json"
            run = subprocess.run(["python3", str(script), "--binary", str(binary), "--seed", "7",
                                  "--duration", "1", "--output", str(report)],
                                 capture_output=True, text=True)
            stderr_record = report.with_suffix(".stderr.txt")
            self.assertNotEqual(run.returncode, 0)
            self.assertIn("retained at", run.stderr)
            self.assertEqual(stderr_record.read_text(), "runtime error: simulated UBSan diagnostic\n")


if __name__ == "__main__":
    unittest.main()
