"""Differential checks for the W5 SVG paint and effect subset."""
import hashlib
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

from tests.svg.shape.png_utils import png_rgba

ROOT = Path(__file__).resolve().parents[3]
TEMP_ROOT = ROOT / ".cache"
TEMP_ROOT.mkdir(exist_ok=True)
CLI = ROOT / "build/mac-debug/bin/shape_render_cli"


class W5ShapeRenderTests(unittest.TestCase):
    def _render_filter_source(self, source, name):
        self.assertTrue(CLI.is_file(), "run ./dev build before the focused filter tests")
        with tempfile.TemporaryDirectory(prefix=f"vexel-{name}-", dir=TEMP_ROOT) as temp:
            fixture = Path(temp) / "case.svg"
            output = Path(temp) / "render.rgba"
            fixture.write_text(source, encoding="utf-8")
            before = hashlib.sha256(fixture.read_bytes()).digest()
            run = subprocess.run([str(CLI), str(fixture), str(output)],
                                 capture_output=True, text=True)
            self.assertEqual(run.returncode, 0, run.stderr)
            self.assertEqual(before, hashlib.sha256(fixture.read_bytes()).digest())
            return output.read_bytes()

    def _render_w5_pair(self, source, name):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by W5 differential cases")
        with tempfile.TemporaryDirectory(prefix=f"vexel-{name}-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            fixture, actual_path, oracle_path = (directory / "case.svg",
                                                  directory / "actual.rgba",
                                                  directory / "oracle.png")
            fixture.write_text(source, encoding="utf-8")
            before = hashlib.sha256(fixture.read_bytes()).digest()
            actual = subprocess.run([str(CLI), str(fixture), str(actual_path)],
                                     capture_output=True, text=True)
            self.assertEqual(actual.returncode, 0, actual.stderr)
            oracle = subprocess.run([renderer, "--format", "png", "--output",
                                     str(oracle_path), str(fixture)],
                                   capture_output=True, text=True)
            self.assertEqual(oracle.returncode, 0, oracle.stderr)
            self.assertEqual(before, hashlib.sha256(fixture.read_bytes()).digest())
            width, height, expected = png_rgba(oracle_path.read_bytes())
            return width, height, actual_path.read_bytes(), expected

    def test_pattern_user_space_tile_and_repeat_matches_librsvg(self):
        source = ('<svg width="16" height="8"><defs><pattern id="p" '
                  'width="4" height="4" patternUnits="userSpaceOnUse">'
                  '<rect width="2" height="4" fill="red"/>'
                  '<rect x="2" width="2" height="4" fill="blue"/>'
                  '</pattern></defs><rect width="16" height="8" '
                  'fill="url(#p)"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "pattern-user-space")
        self.assertEqual((width, height), (16, 8))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        for x, color in ((0, (255, 0, 0, 255)), (1, (255, 0, 0, 255)),
                         (2, (0, 0, 255, 255)), (3, (0, 0, 255, 255)),
                         (4, (255, 0, 0, 255)), (6, (0, 0, 255, 255))):
            self.assertEqual(pixel(actual, x, 2), bytes(color))
            self.assertEqual(pixel(actual, x, 2), pixel(expected, x, 2))

    def test_pattern_object_box_viewbox_and_transform_matches_librsvg(self):
        source = ('<svg width="32" height="16"><defs><pattern id="p" '
                  'x="0" y="0" width="0.25" height="0.5" '
                  'patternUnits="objectBoundingBox" patternTransform="rotate(0)" '
                  'viewBox="0 0 2 2"><rect width="1" height="2" fill="green"/>'
                  '</pattern></defs><rect x="4" y="2" width="24" height="12" '
                  'fill="url(#p)"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "pattern-object-box-viewbox")
        self.assertEqual((width, height), (32, 16))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        for x, y in ((5, 3), (9, 3), (17, 8), (26, 12)):
            self.assertEqual(pixel(actual, x, y), pixel(expected, x, y))

    def test_pattern_stroke_tiles_on_open_path_matches_librsvg(self):
        source = ('<svg width="32" height="16"><defs><pattern id="p" '
                  'width="4" height="4" patternUnits="userSpaceOnUse">'
                  '<rect width="2" height="4" fill="red"/>'
                  '<rect x="2" width="2" height="4" fill="blue"/>'
                  '</pattern></defs><path d="M2 8H30" fill="none" '
                  'stroke="url(#p)" stroke-width="4"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "pattern-stroke")
        self.assertEqual((width, height), (32, 16))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        for x in (2, 4, 6, 8, 10, 12):
            self.assertEqual(pixel(actual, x, 7), pixel(expected, x, 7))

    def test_alpha_mask_region_matches_librsvg(self):
        source = ('<svg width="32" height="16"><defs><mask id="m" '
                  'maskUnits="userSpaceOnUse" maskContentUnits="userSpaceOnUse" '
                  'mask-type="alpha" x="0" y="0" width="16" height="16">'
                  '<rect width="16" height="16" fill="black" fill-opacity="0.5"/>'
                  '</mask></defs><rect width="32" height="16" fill="red" '
                  'mask="url(#m)"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "mask-alpha-region")
        self.assertEqual((width, height), (32, 16))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        self.assertLessEqual(abs(pixel(actual, 4, 8)[3] - 128), 1)
        self.assertEqual(pixel(actual, 20, 8), bytes((0, 0, 0, 0)))
        self.assertLessEqual(abs(pixel(actual, 4, 8)[3] - pixel(expected, 4, 8)[3]), 1)
        self.assertEqual(pixel(actual, 20, 8), pixel(expected, 20, 8))

    def test_luminance_and_nested_mask_match_librsvg(self):
        source = ('<svg width="24" height="16"><defs>'
                  '<mask id="inner" maskUnits="userSpaceOnUse" '
                  'maskContentUnits="userSpaceOnUse" x="0" y="0" width="12" height="16">'
                  '<rect width="6" height="16" fill="white"/></mask>'
                  '<mask id="outer" maskUnits="userSpaceOnUse" '
                  'maskContentUnits="userSpaceOnUse" mask-type="luminance" '
                  'x="0" y="0" width="12" height="16">'
                  '<rect width="12" height="16" fill="red" '
                  'mask="url(#inner)"/></mask></defs>'
                  '<rect width="24" height="16" fill="blue" '
                  'mask="url(#outer)"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "mask-luminance-nested")
        self.assertEqual((width, height), (24, 16))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        self.assertLessEqual(abs(pixel(actual, 3, 8)[3] - 54), 2)
        self.assertEqual(pixel(actual, 9, 8), bytes((0, 0, 0, 0)))
        self.assertEqual(pixel(actual, 16, 8), bytes((0, 0, 0, 0)))
        for x in (3, 9, 16):
            self.assertLessEqual(abs(pixel(actual, x, 8)[3] - pixel(expected, x, 8)[3]), 2)

    def test_group_multiply_blend_matches_librsvg(self):
        source = ('<svg width="24" height="16"><rect width="24" height="16" '
                  'fill="blue"/><g style="mix-blend-mode: multiply; isolation: isolate">'
                  '<rect x="4" y="3" width="12" height="10" fill="red"/>'
                  '</g></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "group-multiply-blend")
        self.assertEqual((width, height), (24, 16))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        self.assertEqual(pixel(actual, 8, 8), bytes((0, 0, 0, 255)))
        self.assertEqual(pixel(actual, 8, 8), pixel(expected, 8, 8))
        self.assertEqual(pixel(actual, 20, 8), pixel(expected, 20, 8))

    def test_paint_order_stroke_then_fill_matches_librsvg(self):
        source = ('<svg width="24" height="24"><rect x="4" y="4" '
                  'width="16" height="16" fill="red" stroke="blue" '
                  'stroke-width="8" paint-order="stroke fill"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "paint-order-stroke-fill")
        self.assertEqual((width, height), (24, 24))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        self.assertEqual(pixel(actual, 5, 12), bytes((255, 0, 0, 255)))
        self.assertEqual(pixel(actual, 5, 12), pixel(expected, 5, 12))
        self.assertEqual(pixel(actual, 12, 12), pixel(expected, 12, 12))

    def test_vector_effect_non_scaling_stroke_width_matches_librsvg(self):
        source = ('<svg width="64" height="48" viewBox="0 0 32 24">'
                  '<path d="M4 8H28" transform="scale(2)" fill="none" '
                  'stroke="black" stroke-width="4" '
                  'vector-effect="non-scaling-stroke"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "non-scaling-stroke")
        self.assertEqual((width, height), (64, 48))
        pixel = lambda image, x, y: image[(y * width + x) * 4 + 3]
        actual_rows = [y for y in range(height) if pixel(actual, 32, y) != 0]
        expected_rows = [y for y in range(height) if pixel(expected, 32, y) != 0]
        self.assertLessEqual(abs(len(actual_rows) - len(expected_rows)), 1)
        self.assertLessEqual(abs(min(actual_rows) - min(expected_rows)), 1)
        self.assertLessEqual(abs(max(actual_rows) - max(expected_rows)), 1)

    def test_crisp_edges_and_extended_linejoin_fallbacks_decode(self):
        cases = (
            '<rect x="0.25" y="0.25" width="8.5" height="8.5" '
            'fill="red" shape-rendering="crispEdges"/>',
            '<polyline points="2,14 8,2 14,14" fill="none" stroke="blue" '
            'stroke-width="3" stroke-linejoin="arcs"/>',
            '<polyline points="18,14 24,2 30,14" fill="none" stroke="green" '
            'stroke-width="3" stroke-linejoin="miter-clip"/>',
        )
        for index, shape in enumerate(cases):
            with self.subTest(index=index):
                rgba = self._render_filter_source(
                    f'<svg width="32" height="16">{shape}</svg>',
                    f"shape-render-hint-{index}")
                self.assertEqual(len(rgba), 32 * 16 * 4)

    def test_marker_start_mid_end_orient_and_ref_matches_librsvg(self):
        source = ('<svg width="40" height="40"><defs>'
                  '<marker id="s" markerWidth="10" markerHeight="10" '
                  'refX="10" refY="5" markerUnits="userSpaceOnUse" '
                  'orient="auto-start-reverse"><path d="M0 0L10 5L0 10z" '
                  'fill="red" stroke="none"/></marker>'
                  '<marker id="m" markerWidth="6" markerHeight="6" '
                  'refX="3" refY="3" markerUnits="userSpaceOnUse" orient="auto">'
                  '<circle cx="3" cy="3" r="3" fill="green" stroke="none"/></marker>'
                  '<marker id="e" markerWidth="10" markerHeight="10" '
                  'refX="10" refY="5" markerUnits="userSpaceOnUse" orient="auto">'
                  '<path d="M0 0L10 5L0 10z" fill="blue" stroke="none"/></marker>'
                  '</defs><path d="M8 16L22 16L22 30" fill="none" stroke="black" '
                  'stroke-width="1" marker-start="url(#s)" marker-mid="url(#m)" '
                  'marker-end="url(#e)"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "markers-start-mid-end")
        self.assertEqual((width, height), (40, 40))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        for x, y, color in ((13, 16, (255, 0, 0, 255)),
                            (22, 16, (0, 128, 0, 255)),
                            (22, 23, (0, 0, 255, 255))):
            self.assertEqual(pixel(actual, x, y), bytes(color))
            self.assertEqual(pixel(actual, x, y), pixel(expected, x, y))

    def test_marker_viewbox_and_stroke_width_units_matches_librsvg(self):
        source = ('<svg width="32" height="32"><defs><marker id="m" '
                  'markerWidth="5" markerHeight="5" refX="10" refY="5" '
                  'markerUnits="strokeWidth" orient="auto" viewBox="0 0 10 10">'
                  '<path d="M0 0L10 5L0 10z" fill="red" stroke="none"/>'
                  '</marker></defs><line x1="4" y1="16" x2="12" y2="16" '
                  'stroke="black" stroke-width="2" marker-end="url(#m)"/></svg>')
        width, height, actual, expected = self._render_w5_pair(
            source, "marker-viewbox-stroke-units")
        self.assertEqual((width, height), (32, 32))
        pixel = lambda image, x, y: image[(y * width + x) * 4:(y * width + x) * 4 + 4]
        self.assertEqual(pixel(actual, 5, 16), bytes((255, 0, 0, 255)))
        self.assertEqual(pixel(actual, 5, 16), pixel(expected, 5, 16))

    def test_marker_shorthand_maps_to_start_mid_and_end(self):
        source = ('<svg width="32" height="24"><defs><marker id="m" '
                  'markerWidth="8" markerHeight="8" refX="8" refY="4" '
                  'markerUnits="userSpaceOnUse" orient="auto"><path '
                  'd="M0 0L8 4L0 8z" fill="red" stroke="none"/>'
                  '</marker></defs><line x1="4" y1="12" x2="24" y2="12" '
                  'stroke="blue" stroke-width="1" marker="url(#m)"/></svg>')
        rgba = self._render_filter_source(source, "marker-shorthand")
        self.assertEqual(rgba[(11 * 32 + 18) * 4:(11 * 32 + 18) * 4 + 4],
                         bytes((255, 0, 0, 255)))

    def test_filter_gaussian_blur_matches_librsvg_with_bounded_tolerance(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by the W5 differential cases")
        source = ('<svg width="32" height="32"><defs><filter id="soft">'
                  '<feGaussianBlur in="SourceGraphic" stdDeviation="2"/>'
                  '</filter></defs><rect x="8" y="8" width="16" height="16" '
                  'fill="red" filter="url(#soft)"/></svg>')
        ours = self._render_filter_source(source, "filter-gaussian-blur")
        with tempfile.TemporaryDirectory(prefix="vexel-filter-blur-", dir=TEMP_ROOT) as temp:
            fixture, oracle = Path(temp) / "blur.svg", Path(temp) / "oracle.png"
            fixture.write_text(source, encoding="utf-8")
            source_hash = hashlib.sha256(fixture.read_bytes()).digest()
            subprocess.run([renderer, "--format", "png", "--output", str(oracle),
                            str(fixture)], check=True, capture_output=True, text=True)
            self.assertEqual(source_hash, hashlib.sha256(fixture.read_bytes()).digest())
            width, height, expected = png_rgba(oracle.read_bytes())
        self.assertEqual((width, height), (32, 32))
        self.assertEqual(len(ours), len(expected))
        maximum_alpha_delta = 0
        for offset in range(0, len(ours), 4):
            maximum_alpha_delta = max(maximum_alpha_delta,
                                      abs(ours[offset + 3] - expected[offset + 3]))
            if ours[offset + 3] != 0 and expected[offset + 3] != 0:
                self.assertEqual(ours[offset:offset + 3], expected[offset:offset + 3])
        self.assertLessEqual(maximum_alpha_delta, 12)
        self.assertGreater(ours[(7 * 32 + 7) * 4 + 3], 0)

    def test_filter_result_chain_offset_and_color_matrix(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by the W5 differential cases")
        source = ('<svg width="32" height="32"><defs><filter id="chain" '
                  'filterUnits="userSpaceOnUse" x="0" y="0" width="32" height="32">'
                  '<feOffset in="SourceGraphic" dx="3" dy="2" result="shift"/>'
                  '<feColorMatrix in="shift" type="matrix" values="'
                  '0 0 0 0 0 0 0 0 0 1 0 0 0 0 0 0 0 0 1 0"/>'
                  '</filter></defs><rect x="5" y="5" width="8" height="8" '
                  'fill="red" filter="url(#chain)"/></svg>')
        rgba = self._render_filter_source(source, "filter-result-chain")
        with tempfile.TemporaryDirectory(prefix="vexel-filter-chain-", dir=TEMP_ROOT) as temp:
            fixture, oracle = Path(temp) / "chain.svg", Path(temp) / "oracle.png"
            fixture.write_text(source, encoding="utf-8")
            subprocess.run([renderer, "--format", "png", "--output", str(oracle),
                            str(fixture)], check=True, capture_output=True, text=True)
            width, height, expected = png_rgba(oracle.read_bytes())
        self.assertEqual((width, height), (32, 32))
        pixel = lambda x, y: rgba[(y * 32 + x) * 4:(y * 32 + x) * 4 + 4]
        self.assertEqual(pixel(10, 10), bytes((0, 255, 0, 255)))
        self.assertEqual(pixel(6, 6), bytes((0, 0, 0, 0)))
        expected_pixel = lambda x, y: expected[(y * width + x) * 4:(y * width + x) * 4 + 4]
        self.assertEqual(pixel(10, 10), expected_pixel(10, 10))
        self.assertEqual(pixel(6, 6), expected_pixel(6, 6))

    def test_filter_turbulence_has_a_stable_named_diagnostic(self):
        source = ('<svg width="16" height="16"><defs><filter id="noise">'
                  '<feTurbulence baseFrequency="0.1" numOctaves="2"/>'
                  '</filter></defs><rect width="16" height="16" '
                  'filter="url(#noise)"/></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-filter-unsupported-",
                                         dir=TEMP_ROOT) as temp:
            fixture = Path(temp) / "unsupported.svg"
            output = Path(temp) / "unsupported.rgba"
            fixture.write_text(source, encoding="utf-8")
            run = subprocess.run([str(CLI), str(fixture), str(output)],
                                 capture_output=True, text=True)
        self.assertNotEqual(run.returncode, 0)
        self.assertEqual(run.stderr.split(":", 1)[0],
                         "unsupported-feTurbulence")

