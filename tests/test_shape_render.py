"""Differential and deterministic integration checks for the bounded shape subset."""
import hashlib
import json
import platform
import random
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib
from pathlib import Path

from tests.svg.shape.paint_clip_generator import random_paint_clip_document

ROOT = Path(__file__).resolve().parents[1]
TEMP_ROOT = ROOT / ".cache"
TEMP_ROOT.mkdir(exist_ok=True)
FIXTURES = ROOT / "tests/svg/shape/fixtures"
CLI = ROOT / "build/mac-debug/bin/shape_render_cli"
REFERENCE_CLI = ROOT / "build/mac-debug/bin/shape_render_reference_cli"
STATS_CLI = ROOT / "build/mac-debug/bin/shape_render_stats_cli"
OUT = ROOT / "artifacts/WO-M4-06/reference-comparison.json"


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


def random_shape_document(rng, index):
    sizes = ((16, 16), (24, 16), (24, 24), (32, 24), (48, 32), (64, 40))
    width, height = sizes[index % len(sizes)]
    fills = ("red", "blue", "green", "#a050d0", "#3080b0")
    caps = ("butt", "square", "round")
    joins = ("miter", "bevel", "round")
    dash_patterns = ("", ' stroke-dasharray="3 2"',
                     ' stroke-dasharray="3 2 1"')
    fragments = []
    for shape_index in range(7):
        x = rng.uniform(1.0, max(2.0, width - 4.0))
        y = rng.uniform(1.0, max(2.0, height - 4.0))
        w = rng.uniform(2.0, max(2.1, min(width / 2.0, 14.0)))
        h = rng.uniform(2.0, max(2.1, min(height / 2.0, 12.0)))
        fill = fills[rng.randrange(len(fills))]
        fill_rule = "evenodd" if (shape_index + index) % 2 else "nonzero"
        kind = (index + shape_index) % 6
        if kind == 0:
            geometry = f'<rect x="{x:.3f}" y="{y:.3f}" width="{w:.3f}" height="{h:.3f}"'
        elif kind == 1:
            geometry = f'<circle cx="{x:.3f}" cy="{y:.3f}" r="{min(w, h) / 2.0:.3f}"'
        elif kind == 2:
            geometry = f'<ellipse cx="{x:.3f}" cy="{y:.3f}" rx="{w / 2.0:.3f}" ry="{h / 2.0:.3f}"'
        elif kind == 3:
            geometry = (f'<polygon points="{x:.3f},{y:.3f} {x+w:.3f},{y+0.5:.3f} '
                        f'{x+w-1:.3f},{y+h:.3f} {x+0.5:.3f},{y+h-1:.3f}"')
        elif kind == 4:
            geometry = (f'<polyline points="{x:.3f},{y+h:.3f} {x+w/2:.3f},{y:.3f} '
                        f'{x+w:.3f},{y+h:.3f}"')
        else:
            d = (f'M {x:.3f} {y+h:.3f} C {x+1:.3f} {y:.3f} {x+w/2:.3f} {y+h:.3f} '
                 f'{x+w/2:.3f} {y:.3f} Q {x+w:.3f} {y-1:.3f} {x+w:.3f} {y+h/2:.3f} '
                 f'A {max(1.0,w/4):.3f} {max(1.0,h/3):.3f} 17 0 1 {x+1:.3f} {y+h/2:.3f} Z')
            geometry = f'<path d="{d}"'
        attrs = f' fill="{fill}" fill-rule="{fill_rule}"'
        if kind == 4:
            attrs = f' fill="none"'
        if shape_index == 0:
            combo = index % 27
            cap = caps[combo % 3]
            join = joins[(combo // 3) % 3]
            dash = dash_patterns[combo // 9]
            attrs += (f' stroke="#000000" stroke-width="{rng.uniform(0.5, 3.5):.3f}"'
                      f' stroke-linecap="{cap}" stroke-linejoin="{join}"'
                      f' stroke-miterlimit="{rng.uniform(1.0, 6.0):.3f}"'
                      f' stroke-dashoffset="{rng.uniform(-2.0, 2.0):.3f}"{dash}')
        element = geometry + attrs + '/>'
        if shape_index % 2 == 0:
            tx, ty = rng.uniform(-2.0, 2.0), rng.uniform(-2.0, 2.0)
            angle = rng.uniform(-25.0, 25.0)
            sx, sy = rng.uniform(0.7, 1.3), rng.uniform(0.7, 1.3)
            element = (f'<g transform="translate({tx:.3f} {ty:.3f}) rotate({angle:.3f}) '
                       f'scale({sx:.3f} {sy:.3f})">{element}</g>')
        fragments.append(element)
    # Guaranteed overlap exercises later-shape replacement with varying alpha.
    fragments.extend((
        '<rect x="4" y="4" width="11" height="9" fill="#d03030"/>',
        '<rect x="8" y="6" width="10" height="9" fill="#2050d0"/>',
    ))
    return (f'<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}">'
            + ''.join(fragments) + '</svg>')


class ShapeRenderTests(unittest.TestCase):
    def _assert_matches_unculled_reference(self, source, name):
        self.assertTrue(CLI.is_file(), "run ./dev build before the focused shape tests")
        self.assertTrue(REFERENCE_CLI.is_file(), "reference renderer must be built for differential tests")
        with tempfile.TemporaryDirectory(prefix=f"vexel-{name}-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            input_path = directory / "case.svg"
            optimized_path = directory / "optimized.rgba"
            reference_path = directory / "reference.rgba"
            input_path.write_text(source, encoding="utf-8")
            optimized = subprocess.run([str(CLI), str(input_path), str(optimized_path)],
                                       capture_output=True, text=True)
            reference = subprocess.run([str(REFERENCE_CLI), str(input_path), str(reference_path)],
                                       capture_output=True, text=True)
            self.assertEqual(optimized.returncode, reference.returncode,
                             f"{optimized.stderr!r} / {reference.stderr!r}")
            if optimized.returncode == 0:
                self.assertEqual(optimized_path.read_bytes(), reference_path.read_bytes())
            else:
                self.assertEqual(optimized.stderr.splitlines()[0],
                                 reference.stderr.splitlines()[0])
            return optimized_path.read_bytes() if optimized.returncode == 0 else b""

    def test_gradient_full_tile_radial_interior_uses_center_sample(self):
        source = ('<svg width="32" height="32"><defs>'
                  '<radialGradient id="g" cx="0.5" cy="0.5" r="0.16">'
                  '<stop offset="0" stop-color="#ff2000" stop-opacity="0.3"/>'
                  '<stop offset="1" stop-color="#0020ff" stop-opacity="0.9"/>'
                  '</radialGradient></defs>'
                  '<rect width="32" height="32" fill="url(#g)"/></svg>')
        rgba = self._assert_matches_unculled_reference(source, "radial-subsamples")
        self.assertTrue(rgba)
        center = (15 * 32 + 15) * 4
        neighbor = (15 * 32 + 14) * 4
        self.assertEqual(rgba[center:center + 4], bytes((220, 32, 35, 98)))
        self.assertNotEqual(rgba[center:center + 4], rgba[neighbor:neighbor + 4])

    def test_user_space_clip_half_pixel_edge_uses_4x4_samples(self):
        source = ('<svg width="32" height="8"><defs>'
                  '<clipPath id="c" clipPathUnits="userSpaceOnUse">'
                  '<rect x="0" y="0" width="20.5" height="8"/>'
                  '</clipPath></defs><rect width="21" height="8" fill="red" '
                  'clip-path="url(#c)"/></svg>')
        rgba = self._assert_matches_unculled_reference(source, "half-pixel-clip")
        pixel = lambda x, y: rgba[(y * 32 + x) * 4:(y * 32 + x) * 4 + 4]
        self.assertEqual(pixel(19, 4), bytes((255, 0, 0, 255)))
        self.assertEqual(pixel(20, 4), bytes((255, 0, 0, 128)))
        self.assertEqual(pixel(21, 4), bytes((0, 0, 0, 0)))

    def test_clip_half_pixel_edge_matches_librsvg_at_fractional_boundary(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by this work order")
        fixture = FIXTURES / "clip-half-pixel-edge.svg"
        with tempfile.TemporaryDirectory(prefix="vexel-clip-edge-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            output = directory / "ours.rgba"
            reference = directory / "reference.png"
            subprocess.run([str(CLI), str(fixture), str(output)], check=True,
                           capture_output=True, text=True)
            subprocess.run([renderer, "--format", "png", "--output", str(reference),
                            str(fixture)], check=True, capture_output=True, text=True)
            width, height, expected = png_rgba(reference.read_bytes())
            self.assertEqual((width, height), (40, 24))
            offset = (4 * width + 20) * 4
            actual_pixel = output.read_bytes()[offset:offset + 4]
            expected_pixel = expected[offset:offset + 4]
            self.assertEqual(expected_pixel, bytes((124, 0, 131, 128)))
            self.assertLessEqual(abs(actual_pixel[3] - 128), 1)
            self.assertLessEqual(max(abs(actual_pixel[channel] - expected_pixel[channel])
                                     for channel in range(3)), 1)

    def test_css_cascade_inheritance_and_source_preservation(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = (b'<svg width="16" height="8" fill="green"><style>/* keep */\n'
                 b'.paint, rect.alt { FiLl: blue; } rect { fill: green; } '
                 b'#target.paint { FILL: RED ! IMPORTANT; } '
                 b'.paint { fill: red !important /* ; } */; } '
                 b'.outer { stroke: blue; stroke-width: 2; }'
                 b'</style><g class="outer"><g><rect id="target" class="paint extra" '
                 b'x="1" y="1" width="4" height="4" fill="white" style="fill: white"/></g>'
                 b'<rect class="paint" x="7" y="1" width="4" height="4" '
                 b'style="fill: blue ! IMPORTANT /* ; } */; fill: white"/>'
                 b'<rect class="plain" x="12" y="1" width="3" height="4" style="fill: white"/></g></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-css-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "style.svg", Path(temp) / "style.rgba"
            input_path.write_bytes(source)
            before = hashlib.sha256(input_path.read_bytes()).digest()
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            rgba = output_path.read_bytes()
            self.assertEqual(before, hashlib.sha256(input_path.read_bytes()).digest())
        pixel = lambda x, y: rgba[(y * 16 + x) * 4:(y * 16 + x) * 4 + 4]
        self.assertEqual(pixel(2, 2), bytes((255, 0, 0, 255)))  # ID beats class and type.
        self.assertEqual(pixel(8, 2), bytes((0, 0, 255, 255)))  # Inline important beats normal.
        self.assertEqual(pixel(13, 2), bytes((255, 255, 255, 255)))  # Inline normal beats stylesheet normal.

    def test_css_invalid_recognized_value_has_value_offset(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = '<svg width="8" height="8"><style>rect { stroke-width: nope; }</style><rect width="4" height="4"/></svg>'
        with tempfile.TemporaryDirectory(prefix="vexel-css-invalid-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "invalid.svg", Path(temp) / "invalid.rgba"
            input_path.write_text(source)
            run = subprocess.run([str(binary), str(input_path), str(output_path)],
                                 capture_output=True, text=True)
        self.assertNotEqual(run.returncode, 0)
        category, offset = run.stderr.strip().split(":", 1)
        self.assertEqual(category, "number")
        self.assertEqual(int(offset), source.index("nope"))

    def test_css_invalid_values_for_all_nine_paint_properties(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        cases = {
            "fill": ("magenta", "unsupported"),
            "fill-rule": ("invalid", "unsupported"),
            "stroke": ("magenta", "unsupported"),
            "stroke-width": ("-1", "number"),
            "stroke-linecap": ("triangle", "unsupported"),
            "stroke-linejoin": ("invalid", "unsupported"),
            "stroke-miterlimit": ("0", "number"),
            "stroke-dasharray": ("0", "number"),
            "stroke-dashoffset": ("wat", "number"),
        }
        with tempfile.TemporaryDirectory(prefix="vexel-css-all-invalid-", dir=TEMP_ROOT) as temp:
            for prop, (value, expected_category) in cases.items():
                source = (f'<svg width="8" height="8"><style>rect {{ {prop}: {value}; }}</style>'
                          '<rect width="4" height="4"/></svg>')
                input_path = Path(temp) / f"{prop}.svg"
                input_path.write_text(source)
                run = subprocess.run([str(binary), str(input_path), str(Path(temp) / "out.rgba")],
                                     capture_output=True, text=True)
                self.assertNotEqual(run.returncode, 0, prop)
                category, offset = run.stderr.strip().split(":", 1)
                self.assertEqual(category, expected_category, prop)
                self.assertEqual(int(offset), source.index(value), prop)

    def test_css_stylesheet_limit_and_unsupported_declarations(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        over_limits = [
            '<style>' + 'x' * (1024 * 1024 + 1) + '</style>',
            '<style>' + 'rect{fill:red;}' * 4097 + '</style>',
            '<style>rect{' + 'fill:red;' * 16385 + '}</style>',
        ]
        with tempfile.TemporaryDirectory(prefix="vexel-css-limit-", dir=TEMP_ROOT) as temp:
            for index, style in enumerate(over_limits):
                input_path, output_path = Path(temp) / f"limit-{index}.svg", Path(temp) / "out.rgba"
                input_path.write_text(f'<svg width="8" height="8">{style}</svg>')
                run = subprocess.run([str(binary), str(input_path), str(output_path)],
                                     capture_output=True, text=True)
                self.assertNotEqual(run.returncode, 0, index)
                self.assertEqual(run.stderr.strip(), "limit:0", index)
        valid = '<svg width="8" height="8"><style>rect { opacity: 0; }</style><rect width="4" height="4"/></svg>'
        with tempfile.TemporaryDirectory(prefix="vexel-css-ignored-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "ignored.svg", Path(temp) / "out.rgba"
            input_path.write_text(valid)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)

    def test_css_class_tokens_and_initial_paint(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = ('<svg width="8" height="8"><style>rect { fill: green; } '
                  '.bar { fill: red; } .unsupported g { fill: blue; }</style>'
                  '<rect class="foobar" width="3" height="3"/>'
                  '<rect x="4" width="3" height="3"/></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-css-class-tokens-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "tokens.svg", Path(temp) / "tokens.rgba"
            input_path.write_text(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            rgba = output_path.read_bytes()
        pixel = lambda x, y: rgba[(y * 8 + x) * 4:(y * 8 + x) * 4 + 4]
        self.assertEqual(pixel(1, 1), bytes((0, 128, 0, 255)))  # .bar does not match foobar.
        self.assertEqual(pixel(5, 1), bytes((0, 128, 0, 255)))
        default_source = '<svg width="8" height="8"><rect width="3" height="3"/></svg>'
        with tempfile.TemporaryDirectory(prefix="vexel-css-initial-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "default.svg", Path(temp) / "default.rgba"
            input_path.write_text(default_source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            default_rgba = output_path.read_bytes()
        self.assertEqual(default_rgba[(1 * 8 + 1) * 4:(1 * 8 + 1) * 4 + 4],
                         bytes((0, 0, 0, 255)))
        many_tokens = " ".join([f"c{i}" for i in range(256)] + ["target"])
        many_terms = "." + ".".join(f"a{i}" for i in range(17))
        bounded = (f'<svg width="8" height="8"><style>rect {{ fill: green; }} '
                   f'.target {{ fill: red; }} {many_terms} {{ fill: blue; }}</style>'
                   f'<rect class="{many_tokens}" width="3" height="3"/></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-css-bounded-classes-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "bounded.svg", Path(temp) / "bounded.rgba"
            input_path.write_text(bounded)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            bounded_rgba = output_path.read_bytes()
        self.assertEqual(bounded_rgba[(1 * 8 + 1) * 4:(1 * 8 + 1) * 4 + 4],
                         bytes((0, 128, 0, 255)))

    def test_css_presentation_precedence_source_order_and_inheritance(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = (b'<svg width="16" height="8" fill="blue"><style>'
                 b'.direct { fill: red; } .same { fill: red; } .same { fill: green; } '
                 b'.parent { fill: white; }</style>'
                 b'<rect x="0" y="0" width="3" height="3"/>'
                 b'<g class="parent"><g><rect x="4" y="0" width="3" height="3"/></g></g>'
                 b'<rect class="direct" fill="blue" x="8" y="0" width="3" height="3"/>'
                 b'<rect class="same" x="12" y="0" width="3" height="3"/></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-css-precedence-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "precedence.svg", Path(temp) / "precedence.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            rgba = output_path.read_bytes()
        pixel = lambda x, y: rgba[(y * 16 + x) * 4:(y * 16 + x) * 4 + 4]
        self.assertEqual(pixel(1, 1), bytes((0, 0, 255, 255)))  # Root paint is inherited.
        self.assertEqual(pixel(5, 1), bytes((255, 255, 255, 255)))  # Nested group paint is inherited.
        self.assertEqual(pixel(9, 1), bytes((255, 0, 0, 255)))  # Rule beats presentation attribute.
        self.assertEqual(pixel(13, 1), bytes((0, 128, 0, 255)))  # Later equal-specificity rule wins.

    def test_css_rule_match_work_limit(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        rules = "".join(f".c{index} {{ fill: red; }}" for index in range(2048))
        shapes = '<rect width="1" height="1"/>' * 2050
        source = f'<svg width="8" height="8"><style>{rules}</style>{shapes}</svg>'
        with tempfile.TemporaryDirectory(prefix="vexel-css-match-limit-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "limit.svg", Path(temp) / "limit.rgba"
            input_path.write_text(source)
            run = subprocess.run([str(binary), str(input_path), str(output_path)],
                                 capture_output=True, text=True)
        self.assertNotEqual(run.returncode, 0)
        self.assertEqual(run.stderr.strip(), "limit:0")

    def test_decoder_diagnostics_source_retention_and_raster(self):
        self.assertTrue(CLI.is_file(), "run ./dev build before the focused shape tests")
        result = subprocess.run([str(ROOT / "build/mac-debug/bin/shape_test")], check=True, capture_output=True, text=True)
        self.assertIn("shape decoder and 4x4 antialiased raster checks passed", result.stdout)

    def test_reference_comparison_and_repeatability(self):
        renderer = shutil.which("rsvg-convert")
        self.assertIsNotNone(renderer, "rsvg-convert is required by this work order")
        self.assertTrue(REFERENCE_CLI.is_file(), "reference renderer must be built for differential tests")
        version = subprocess.run([renderer, "--version"], check=True, capture_output=True, text=True).stdout.strip()
        scalar_identity_cases = {
            "clip-union", "object-bbox-transform", "user-space-transform",
            "gradient-stop-alpha-overpaint",
        }
        results = []
        failed_fixtures = []
        with tempfile.TemporaryDirectory(prefix="vexel-shape-", dir=TEMP_ROOT) as temp:
            temp_path = Path(temp)
            for fixture in sorted(FIXTURES.glob("*.svg")):
                raw = fixture.read_bytes()
                source_hash = hashlib.sha256(raw).digest()
                first, second, reference = (temp_path / f"{fixture.stem}-{i}.rgba" for i in range(3))
                subprocess.run([str(CLI), str(fixture), str(first)], check=True, capture_output=True)
                subprocess.run([str(CLI), str(fixture), str(second)], check=True, capture_output=True)
                self.assertEqual(first.read_bytes(), second.read_bytes(), fixture.name)
                if fixture.stem in scalar_identity_cases:
                    scalar_reference = temp_path / f"{fixture.stem}-unculled.rgba"
                    subprocess.run([str(REFERENCE_CLI), str(fixture), str(scalar_reference)],
                                   check=True, capture_output=True)
                    self.assertEqual(first.read_bytes(), scalar_reference.read_bytes(),
                                     f"culled and unculled renderers diverged for {fixture.name}")
                subprocess.run([renderer, "--format", "png", "--output", str(reference), str(fixture)], check=True, capture_output=True)
                self.assertEqual(source_hash, hashlib.sha256(fixture.read_bytes()).digest(), fixture.name)
                rw, rh, reference_rgba = png_rgba(reference.read_bytes())
                ours = first.read_bytes()
                width, height = rw, rh
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
                failures = []
                if bad_interior != 0:
                    failures.append(f"interior_diff_pixels={bad_interior} (limit 0)")
                if edge_diffs > 120:
                    failures.append(f"edge_diff_pixels={edge_diffs} (limit 120)")
                if max_alpha_delta > 160:
                    failures.append(f"max_alpha_diff={max_alpha_delta} (limit 160)")
                if max_rgb_delta > 1:
                    failures.append(f"max_jointly_covered_straight_rgb_diff={max_rgb_delta} (limit 1)")
                if failures:
                    failed_fixtures.append({"case": fixture.stem, "failures": failures})
                results.append({"case": fixture.stem, "source_sha256": hashlib.sha256(raw).hexdigest(),
                                "vexel_sha256": hashlib.sha256(ours).hexdigest(),
                                "reference_png_sha256": hashlib.sha256(reference.read_bytes()).hexdigest(),
                                "edge_diff_pixels": edge_diffs, "interior_diff_pixels": bad_interior,
                                "max_channel_diff": max_channel,
                                "max_jointly_covered_straight_rgb_diff": max_rgb_delta,
                                "max_alpha_diff": max_alpha_delta,
                                "threshold_status": "fail" if failures else "pass",
                                "threshold_failures": failures})
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
                "cases": results, "failed_fixtures": failed_fixtures,
                "status": "fail" if failed_fixtures else "pass"}, indent=2) + "\n")
        self.assertEqual(failed_fixtures, [],
                         "librsvg fixture threshold failures are recorded in " + str(OUT))

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
        with tempfile.TemporaryDirectory(prefix="vexel-shape-bounds-", dir=TEMP_ROOT) as temp:
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
        with tempfile.TemporaryDirectory(prefix="vexel-shape-order-", dir=TEMP_ROOT) as temp:
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
        with tempfile.TemporaryDirectory(prefix="vexel-shape-fractional-", dir=TEMP_ROOT) as temp:
            input_path, output_path = Path(temp) / "fractional.svg", Path(temp) / "fractional.rgba"
            input_path.write_bytes(source)
            subprocess.run([str(binary), str(input_path), str(output_path)], check=True,
                           capture_output=True)
            self.assertEqual(output_path.stat().st_size, 9 * 8 * 4)

    def test_default_viewbox_meet_letterboxes_mismatched_aspect_ratio(self):
        binary = ROOT / "build/mac-debug/bin/shape_render_cli"
        source = (b'<svg width="16" height="16" viewBox="0 0 32 16">'
                  b'<rect x="0" y="0" width="32" height="16"/></svg>')
        with tempfile.TemporaryDirectory(prefix="vexel-viewbox-meet-", dir=TEMP_ROOT) as temp:
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
        with tempfile.TemporaryDirectory(prefix="vexel-height-offset-", dir=TEMP_ROOT) as temp:
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
        with tempfile.TemporaryDirectory(prefix="vexel-transform-offset-", dir=TEMP_ROOT) as temp:
            input_path = Path(temp) / "offset.svg"
            output_path = Path(temp) / "offset.rgba"
            input_path.write_bytes(source)
            run = subprocess.run([str(binary), str(input_path), str(output_path)], capture_output=True, text=True)
        category, offset = run.stderr.strip().split(":", 1)
        self.assertNotEqual(run.returncode, 0)
        self.assertEqual(category, "unsupported")
        self.assertEqual(int(offset), source.index(b"skewX"))

    def test_culled_renderer_matches_reference_for_fixtures_and_random_documents(self):
        self.assertTrue(CLI.is_file(), "run ./dev build before the focused shape tests")
        self.assertTrue(REFERENCE_CLI.is_file(), "reference renderer must be built for differential tests")
        self.assertTrue(STATS_CLI.is_file(), "stats renderer must be built for differential tests")
        seed = 20261011
        feature_seed = 20261013
        generated_count = 200
        feature_generated_count = 160
        rng = random.Random(seed)
        feature_rng = random.Random(feature_seed)
        oracle_sources = list(sorted(FIXTURES.glob("*.svg")))
        oracle_sources.extend(sorted((ROOT / "tests/viewer/fixtures").glob("*.svg")))
        sources = [path.read_bytes() for path in oracle_sources]
        generated = [random_shape_document(rng, index).encode("utf-8")
                     for index in range(generated_count)]
        feature_generated = [random_paint_clip_document(feature_rng, index).encode("utf-8")
                             for index in range(feature_generated_count)]
        print(f"raster differential seed={seed} generated={generated_count} "
              f"paint-clip-seed={feature_seed} paint-clip-generated={feature_generated_count} "
              "tile-sizes=16x16,24x16,24x24,32x24,48x32,64x40")
        with tempfile.TemporaryDirectory(prefix="vexel-culling-diff-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            for index, source in enumerate(sources + generated + feature_generated):
                input_path = directory / f"case-{index}.svg"
                optimized_path = directory / f"case-{index}.optimized.rgba"
                reference_path = directory / f"case-{index}.reference.rgba"
                input_path.write_bytes(source)
                is_generated = index >= len(sources)
                is_feature_generated = index >= len(sources) + generated_count
                if is_generated and not is_feature_generated:
                    optimized = subprocess.run(
                        [str(STATS_CLI), "--translucent", str(input_path), str(optimized_path)],
                        capture_output=True, text=True)
                    reference = subprocess.run(
                        [str(REFERENCE_CLI), "--translucent", str(input_path), str(reference_path)],
                        capture_output=True, text=True)
                    case = f"seed={seed} document={index-len(sources)}"
                elif is_feature_generated:
                    optimized = subprocess.run(
                        [str(STATS_CLI), str(input_path), str(optimized_path)],
                        capture_output=True, text=True)
                    reference = subprocess.run(
                        [str(REFERENCE_CLI), str(input_path), str(reference_path)],
                        capture_output=True, text=True)
                    case = (f"paint-clip-seed={feature_seed} document="
                            f"{index-len(sources)-generated_count}")
                else:
                    optimized = subprocess.run(
                        [str(CLI), str(input_path), str(optimized_path)],
                        capture_output=True, text=True)
                    reference = subprocess.run(
                        [str(REFERENCE_CLI), str(input_path), str(reference_path)],
                        capture_output=True, text=True)
                    case = input_path.name
                with self.subTest(case=case):
                    self.assertEqual(optimized.returncode, reference.returncode,
                                     f"renderer status differs: {optimized.stderr!r} / {reference.stderr!r}")
                    if optimized.returncode == 0:
                        self.assertEqual(optimized_path.read_bytes(), reference_path.read_bytes())
                    else:
                        optimized_error = optimized.stderr.splitlines()[0] if optimized.stderr else ""
                        reference_error = reference.stderr.splitlines()[0] if reference.stderr else ""
                        self.assertEqual(optimized_error, reference_error)

    def test_culling_renders_large_shapes_under_the_existing_work_cap(self):
        self.assertTrue(STATS_CLI.is_file(), "run ./dev build before the focused shape tests")
        circle_sources = (
            '<svg width="640" height="400"><circle cx="170" cy="200" r="110" fill="red"/></svg>',
            '<svg width="640" height="400"><circle cx="170" cy="200" r="150" fill="red"/></svg>',
        )
        many_shapes = ''.join(
            f'<rect x="{(index % 100) * 18 + 2}" y="{(index // 100) * 20 + 2}" '
            'width="5" height="5" fill="#336699"/>'
            for index in range(5000))
        circle_sources += (f'<svg width="1920" height="1080">{many_shapes}</svg>',)
        with tempfile.TemporaryDirectory(prefix="vexel-culling-large-", dir=TEMP_ROOT) as temp:
            directory = Path(temp)
            for index, source in enumerate(circle_sources):
                input_path = directory / f"large-{index}.svg"
                output_path = directory / f"large-{index}.rgba"
                input_path.write_text(source)
                result = subprocess.run([str(STATS_CLI), str(input_path), str(output_path)],
                                        capture_output=True, text=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(output_path.stat().st_size,
                                 (640 * 400 * 4) if index < 2 else (1920 * 1080 * 4))
                counters = {key: int(value) for key, value in
                            (line.split("=", 1) for line in result.stderr.splitlines())}
                self.assertGreater(counters["shape_pixels"], 0)
                self.assertGreater(counters["fill_edge_tests"], 0)
                self.assertLess(counters["total_work"], 200_000_000)
                if index == 2:
                    self.assertLess(counters["shape_pixels"], 1_000_000)
                    self.assertLess(counters["shape_row_visits"], 100_000)

            clipped = ('<svg width="64" height="40"><defs>'
                       '<clipPath id="left"><rect width="20" height="40"/></clipPath>'
                       '<linearGradient id="half-blue">'
                       '<stop stop-color="blue" stop-opacity="0.5"/>'
                       '<stop offset="1" stop-color="blue" stop-opacity="0.5"/>'
                       '</linearGradient></defs>'
                       '<rect width="64" height="40" fill="red"/>'
                       '<rect width="64" height="40" fill="url(#half-blue)" '
                       'clip-path="url(#left)"/></svg>')
            input_path, output_path = directory / "clipped-gradient.svg", directory / "clipped-gradient.rgba"
            input_path.write_text(clipped)
            result = subprocess.run([str(STATS_CLI), str(input_path), str(output_path)],
                                    capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            counters = {key: int(value) for key, value in
                        (line.split("=", 1) for line in result.stderr.splitlines())}
            self.assertEqual(counters["shape_pixels"], 2 * 64 * 40)
            self.assertLess(counters["total_work"], 200_000_000)
            pixels = output_path.read_bytes()
            pixel = lambda x, y: pixels[(y * 64 + x) * 4:(y * 64 + x) * 4 + 4]
            self.assertEqual(pixel(10, 20), bytes((127, 0, 128, 255)))
            self.assertEqual(pixel(40, 20), bytes((255, 0, 0, 255)))

    def test_shape_fuzz_wrapper_rejects_and_retains_sanitizer_stderr(self):
        script = ROOT / "tests/fuzz/shape/shape_fuzz.py"
        with tempfile.TemporaryDirectory(prefix="vexel-fuzz-stderr-", dir=TEMP_ROOT) as temp:
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
