"""Pixel spot checks against librsvg's static render of sampled values."""

from __future__ import annotations

import math
from pathlib import Path
import re
import shutil
import struct
import subprocess
import tempfile
import unittest
import zlib


ROOT = Path(__file__).resolve().parents[2]
ANIMATED_RENDER = ROOT / "build" / "mac-debug" / "bin" / "animation_render_cli"
FIXTURE = ROOT / "tests" / "anim" / "fixtures" / "slide-x.svg"
BLUE = bytes((79, 124, 255, 255))


def decode_png(path: Path) -> tuple[int, int, bytes]:
    data = path.read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise AssertionError("rsvg-convert did not write a PNG")
    cursor = 8
    compressed = bytearray()
    width = height = color_type = bit_depth = interlace = 0
    while cursor + 12 <= len(data):
        length = struct.unpack_from(">I", data, cursor)[0]
        tag = data[cursor + 4:cursor + 8]
        payload_start = cursor + 8
        payload_end = payload_start + length
        if payload_end + 4 > len(data):
            raise AssertionError("truncated PNG chunk")
        payload = data[payload_start:payload_end]
        if tag == b"IHDR":
            width, height, bit_depth, color_type, _compression, _filter, interlace = struct.unpack(
                ">IIBBBBB", payload
            )
        elif tag == b"IDAT":
            compressed.extend(payload)
        elif tag == b"IEND":
            break
        cursor = payload_end + 4
    if bit_depth != 8 or color_type not in (2, 6) or interlace != 0:
        raise AssertionError("cross-check PNG uses an unsupported pixel encoding")
    channels = 4 if color_type == 6 else 3
    stride = width * channels
    raw = zlib.decompress(compressed)
    if len(raw) != height * (stride + 1):
        raise AssertionError("decoded PNG rows have an unexpected length")
    pixels = bytearray(width * height * 4)
    previous = bytearray(stride)
    input_offset = 0
    for y in range(height):
        filter_type = raw[input_offset]
        input_offset += 1
        row = bytearray(raw[input_offset:input_offset + stride])
        input_offset += stride
        for x in range(stride):
            left = row[x - channels] if x >= channels else 0
            above = previous[x]
            upper_left = previous[x - channels] if x >= channels else 0
            if filter_type == 1:
                row[x] = (row[x] + left) & 255
            elif filter_type == 2:
                row[x] = (row[x] + above) & 255
            elif filter_type == 3:
                row[x] = (row[x] + ((left + above) // 2)) & 255
            elif filter_type == 4:
                base = left + above - upper_left
                distances = (abs(base - left), abs(base - above), abs(base - upper_left))
                predictor = left if distances[0] <= distances[1] and distances[0] <= distances[2] else (
                    above if distances[1] <= distances[2] else upper_left
                )
                row[x] = (row[x] + predictor) & 255
            elif filter_type != 0:
                raise AssertionError("unsupported PNG row filter")
        for x in range(width):
            source = x * channels
            target = (y * width + x) * 4
            pixels[target:target + 3] = row[source:source + 3]
            pixels[target + 3] = row[source + 3] if channels == 4 else 255
        previous = row
    return width, height, bytes(pixels)


def matrix_compose(left: tuple[float, ...], right: tuple[float, ...]) -> tuple[float, ...]:
    a, b, c, d, e, f = left
    g, h, i, j, k, ell = right
    return (a * g + c * h, b * g + d * h, a * i + c * j,
            b * i + d * j, a * k + c * ell + e, b * k + d * ell + f)


def matrix_for(name: str, values: tuple[float, ...]) -> tuple[float, ...]:
    if name == "translate":
        x, y = (values + (0.0,))[:2]
        return (1.0, 0.0, 0.0, 1.0, x, y)
    if name == "scale":
        x, y = (values + (values[0],))[:2]
        return (x, 0.0, 0.0, y, 0.0, 0.0)
    if name == "rotate":
        angle = math.radians(values[0])
        cosine, sine = math.cos(angle), math.sin(angle)
        rotation = (cosine, sine, -sine, cosine, 0.0, 0.0)
        if len(values) < 3:
            return rotation
        cx, cy = values[1], values[2]
        return matrix_compose(
            matrix_compose(matrix_for("translate", (cx, cy)), rotation),
            matrix_for("translate", (-cx, -cy)),
        )
    if name in ("skewX", "skewY"):
        tangent = math.tan(math.radians(values[0]))
        return (1.0, 0.0, tangent, 1.0, 0.0, 0.0) if name == "skewX" else (
            1.0, tangent, 0.0, 1.0, 0.0, 0.0
        )
    raise AssertionError(f"unknown transform type {name}")


def parse_transform_list(value: str) -> tuple[float, ...]:
    result = (1.0, 0.0, 0.0, 1.0, 0.0, 0.0)
    for name, raw_values in re.findall(r"([A-Za-z]+)\(([^)]*)\)", value):
        values = tuple(float(part) for part in re.split(r"[ ,]+", raw_values.strip()) if part)
        result = matrix_compose(result, matrix_for(name, values))
    return result


def matrix_attribute(matrix: tuple[float, ...]) -> str:
    return "matrix(" + " ".join(f"{value:.12g}" for value in matrix) + ")"


def interpolated_transform(name: str, progress: float) -> tuple[float, ...]:
    endpoints = {
        "translate": ((0.0, 0.0), (8.0, 4.0)),
        "scale": ((1.0, 1.0), (1.2, 0.8)),
        "rotate": ((0.0,), (90.0,)),
        "skewX": ((0.0,), (12.0,)),
        "skewY": ((0.0,), (-10.0,)),
    }
    first, last = endpoints[name]
    values = tuple(a + (b - a) * progress for a, b in zip(first, last))
    return matrix_for(name, values)


def transform_case_sources(
    mode: str, parent: str, transform_type: str, static_transform: str,
    additive: bool, seconds: float,
) -> tuple[str, str, tuple[float, float]]:
    progress = min(1.0, max(0.0, seconds / 2.0))
    animated_matrix = interpolated_transform(transform_type, progress)
    static_matrix = parse_transform_list(static_transform)
    target_matrix = (
        matrix_compose(static_matrix, animated_matrix) if additive else animated_matrix
    )
    target_attribute = f" transform='{static_transform}'" if static_transform else ""
    oracle_attribute = f" transform='{matrix_attribute(target_matrix)}'"
    additive_attribute = " additive='sum'" if additive else ""
    animation = (
        f"<animateTransform attributeName='transform' type='{transform_type}' "
        f"from='{ {'translate': '0 0', 'scale': '1 1', 'rotate': '0', 'skewX': '0', 'skewY': '0'}[transform_type] }' "
        f"to='{ {'translate': '8 4', 'scale': '1.2 0.8', 'rotate': '90', 'skewX': '12', 'skewY': '-10'}[transform_type] }' "
        f"dur='2s' fill='freeze'{additive_attribute}/>"
    )
    root_start = (
        "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
        f"<g transform='translate(58 42) {parent}'>"
    )
    root_end = "</g></svg>"
    shape = "<rect x='-12' y='-8' width='24' height='16' fill='#4f7cff'/>"
    if mode == "direct":
        animated_target = shape[:-2] + target_attribute + ">" + animation + "</rect>"
        static_target = shape[:-2] + oracle_attribute + "/>"
        animated = root_start + "<g transform='translate(3 2)'>{}</g>".format(animated_target) + root_end
        static = root_start + "<g transform='translate(3 2)'>{}</g>".format(static_target) + root_end
        placement = matrix_compose(parse_transform_list(parent), parse_transform_list("translate(3 2)"))
    elif mode == "group":
        animated = root_start + f"<g transform='{static_transform}'>{animation}{shape}</g>" + root_end
        static = root_start + f"<g{oracle_attribute}>{shape}</g>" + root_end
        placement = parse_transform_list(parent)
    elif mode == "use":
        animated_target = shape[:-2] + target_attribute + ">" + animation + "</rect>"
        static_target = shape[:-2] + oracle_attribute + "/>"
        animated = root_start + (
            "<defs><g id='asset' transform='translate(2 1)'>" +
            animated_target + "</g></defs><use href='#asset' x='1' y='2' "
            "transform='translate(5 3)'/>" + root_end
        )
        static = root_start + (
            "<defs><g id='asset' transform='translate(2 1)'>" +
            static_target + "</g></defs><use href='#asset' x='1' y='2' "
            "transform='translate(5 3)'/>" + root_end
        )
        placement = matrix_compose(
            parse_transform_list(parent),
            matrix_compose(parse_transform_list("translate(5 3)"),
                           matrix_compose(parse_transform_list("translate(1 2)"),
                                          parse_transform_list("translate(2 1)"))),
        )
    elif mode == "symbol":
        animated_target = shape[:-2] + target_attribute + ">" + animation + "</rect>"
        static_target = shape[:-2] + oracle_attribute + "/>"
        prefix = "<defs><symbol id='asset' viewBox='-32 -24 64 48'><g transform='translate(2 1)'>"
        suffix = "</g></symbol></defs><use href='#asset' x='-32' y='-24' width='64' height='48' transform='translate(5 3)'/>"
        animated = root_start + prefix + animated_target + suffix + root_end
        static = root_start + prefix + static_target + suffix + root_end
        placement = matrix_compose(
            parse_transform_list(parent),
            matrix_compose(parse_transform_list("translate(5 3)"),
                           parse_transform_list("translate(2 1)")),
        )
    elif mode == "nested-svg":
        animated_target = shape[:-2] + target_attribute + ">" + animation + "</rect>"
        static_target = shape[:-2] + oracle_attribute + "/>"
        prefix = "<svg x='-32' y='-24' width='64' height='48' viewBox='-32 -24 64 48' transform='translate(5 3)'><g transform='translate(2 1)'>"
        suffix = "</g></svg>"
        animated = root_start + prefix + animated_target + suffix + root_end
        static = root_start + prefix + static_target + suffix + root_end
        placement = matrix_compose(
            parse_transform_list(parent),
            matrix_compose(parse_transform_list("translate(5 3)"),
                           parse_transform_list("translate(2 1)")),
        )
    else:
        raise AssertionError(f"unknown hierarchy mode {mode}")
    complete = matrix_compose(
        matrix_compose(parse_transform_list("translate(58 42)"), placement),
        target_matrix,
    )
    center_x = complete[4]
    center_y = complete[5]
    return animated, static, (center_x, center_y)


class RenderReferenceTests(unittest.TestCase):
    def test_nested_transform_matrix_matches_python_and_librsvg(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        parents = (
            "translate(5 3)", "scale(0.9 1.1)", "rotate(12)",
            "skewX(8)", "skewY(-6)",
        )
        statics = ("translate(2 1)", "scale(1.1 0.9)", "rotate(15)", "skewX(5)")
        transform_types = ("translate", "scale", "rotate", "skewX", "skewY")
        modes = ("direct", "group", "use", "symbol", "nested-svg")
        cases = []
        for parent_index, parent in enumerate(parents):
            for transform_index, transform_type in enumerate(transform_types):
                mode = modes[(parent_index + transform_index) % len(modes)]
                cases.append((mode, parent, transform_type,
                              statics[(parent_index + transform_index) % len(statics)],
                              (parent_index + transform_index) % 3 == 0))
        for index in range(5):
            cases.append((modes[index], parents[(index + 2) % len(parents)],
                          transform_types[(index + 1) % len(transform_types)],
                          statics[(index + 1) % len(statics)], True))
        self.assertGreaterEqual(len(cases), 30)

        with tempfile.TemporaryDirectory(prefix="anim-hierarchy-matrix-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            for case_number, case in enumerate(cases):
                for seconds in (0.5, 1.0):
                    animated, static, center = transform_case_sources(*case, seconds)
                    animated_path.write_text(animated, encoding="utf-8")
                    static_path.write_text(static, encoding="utf-8")
                    subprocess.run(
                        [str(ANIMATED_RENDER), str(animated_path),
                         f"{seconds:.17g}", str(actual_path)],
                        cwd=ROOT, check=True, capture_output=True, timeout=20,
                    )
                    subprocess.run(
                        [str(converter), "--width", "128", "--height", "96",
                         "--output", str(expected_path), str(static_path)],
                        cwd=ROOT, check=True, capture_output=True, timeout=20,
                    )
                    width, height, oracle = decode_png(expected_path)
                    self.assertEqual((width, height), (128, 96))
                    x, y = math.floor(center[0]), math.floor(center[1])
                    self.assertGreaterEqual(x, 0, f"case {case_number} reference x")
                    self.assertGreaterEqual(y, 0, f"case {case_number} reference y")
                    self.assertLess(x, width, f"case {case_number} reference x")
                    self.assertLess(y, height, f"case {case_number} reference y")
                    render = actual_path.read_bytes()
                    self.assertEqual(len(render), width * height * 4)
                    offset = (y * width + x) * 4
                    self.assertEqual(
                        render[offset:offset + 4], oracle[offset:offset + 4],
                        f"case {case_number}, {case}, t={seconds}, "
                        f"Python center=({center[0]:.3f},{center[1]:.3f})",
                    )
                    self.assertEqual(oracle[offset:offset + 4], BLUE,
                                     f"case {case_number} reference center is uncovered")

    def test_animated_geometry_values_use_local_user_coordinates(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        parent = "translate(58 42) rotate(15) skewX(4)"
        child = "translate(2 1) scale(1.1 0.9)"
        cases = (
            ("rect", "x", "-10", "2", (15.0, 0.0)),
            ("rect", "y", "-8", "4", (0.0, 10.0)),
            ("rect", "width", "20", "40", (15.0, 4.0)),
            ("rect", "height", "16", "32", (4.0, 12.0)),
            ("circle", "cx", "-8", "4", (5.0, 0.0)),
            ("circle", "r", "8", "16", (3.0, 0.0)),
        )
        with tempfile.TemporaryDirectory(prefix="anim-geometry-local-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            for tag, attribute, start, finish, local_probe in cases:
                if tag == "rect":
                    base_attrs = "x='-10' y='-8' width='20' height='16'"
                else:
                    base_attrs = "cx='-8' cy='0' r='8'"
                animated = (
                    "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
                    f"<g transform='{parent}'><{tag} {base_attrs} transform='{child}' "
                    "fill='#4f7cff'>"
                    f"<animate attributeName='{attribute}' from='{start}' to='{finish}' "
                    "dur='2s' fill='freeze'/>"
                    f"</{tag}></g></svg>"
                )
                value = float(start) + (float(finish) - float(start)) * 0.5
                static_attrs = base_attrs.replace(
                    f"{attribute}='{start}'", f"{attribute}='{value:.12g}'"
                )
                static = (
                    "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
                    f"<g transform='{parent}'><{tag} {static_attrs} transform='{child}' "
                    f"fill='#4f7cff'/></g></svg>"
                )
                animated_path.write_text(animated, encoding="utf-8")
                static_path.write_text(static, encoding="utf-8")
                subprocess.run(
                    [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                subprocess.run(
                    [str(converter), "--width", "128", "--height", "96",
                     "--output", str(expected_path), str(static_path)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                width, height, oracle = decode_png(expected_path)
                global_transform = matrix_compose(
                    parse_transform_list(parent), parse_transform_list(child)
                )
                probe = matrix_compose(
                    global_transform,
                    (1.0, 0.0, 0.0, 1.0, local_probe[0], local_probe[1]),
                )
                x, y = math.floor(probe[4]), math.floor(probe[5])
                offset = (y * width + x) * 4
                render = actual_path.read_bytes()
                self.assertEqual(render[offset:offset + 4], oracle[offset:offset + 4],
                                 f"{tag} {attribute} local probe={local_probe}")
                self.assertEqual(oracle[offset:offset + 4], BLUE,
                                 f"reference does not cover {tag} {attribute} probe")

    def test_animate_motion_composes_through_group_and_use_ancestors(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        modes = ("group", "use", "symbol", "nested-svg")
        parent = "translate(58 42) rotate(10) scale(0.9 1.1)"
        static_child = parse_transform_list("rotate(15)")
        motion = matrix_for("translate", (6.0, 0.0))
        expected_target = matrix_compose(motion, static_child)
        with tempfile.TemporaryDirectory(prefix="anim-motion-ancestor-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            for mode in modes:
                target = (
                    "<rect x='-12' y='-8' width='24' height='16' fill='#4f7cff' "
                    "transform='rotate(15)'><animateMotion path='M0 0 L12 0' "
                    "dur='2s' fill='freeze'/></rect>"
                )
                static_target = (
                    "<rect x='-12' y='-8' width='24' height='16' fill='#4f7cff' "
                    f"transform='{matrix_attribute(expected_target)}'/>"
                )
                if mode == "group":
                    animated_inner = f"<g transform='translate(2 1)'>{target}</g>"
                    static_inner = f"<g transform='translate(2 1)'>{static_target}</g>"
                    placement = parse_transform_list("translate(2 1)")
                elif mode == "use":
                    animated_inner = "<defs><g id='asset' transform='translate(2 1)'>" + target + "</g></defs><use href='#asset' transform='translate(4 2)'/>"
                    static_inner = "<defs><g id='asset' transform='translate(2 1)'>" + static_target + "</g></defs><use href='#asset' transform='translate(4 2)'/>"
                    placement = matrix_compose(parse_transform_list("translate(4 2)"),
                                               parse_transform_list("translate(2 1)"))
                elif mode == "symbol":
                    prefix = "<defs><symbol id='asset' viewBox='-32 -24 64 48'><g transform='translate(2 1)'>"
                    suffix = "</g></symbol></defs><use href='#asset' x='-32' y='-24' width='64' height='48' transform='translate(4 2)'/>"
                    animated_inner, static_inner = prefix + target + suffix, prefix + static_target + suffix
                    placement = matrix_compose(parse_transform_list("translate(4 2)"),
                                               parse_transform_list("translate(2 1)"))
                else:
                    prefix = "<svg x='-32' y='-24' width='64' height='48' viewBox='-32 -24 64 48' transform='translate(4 2)'><g transform='translate(2 1)'>"
                    suffix = "</g></svg>"
                    animated_inner, static_inner = prefix + target + suffix, prefix + static_target + suffix
                    placement = matrix_compose(parse_transform_list("translate(4 2)"),
                                               parse_transform_list("translate(2 1)"))
                animated = (
                    "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
                    f"<g transform='{parent}'>{animated_inner}</g></svg>"
                )
                static = (
                    "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
                    f"<g transform='{parent}'>{static_inner}</g></svg>"
                )
                animated_path.write_text(animated, encoding="utf-8")
                static_path.write_text(static, encoding="utf-8")
                subprocess.run(
                    [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                subprocess.run(
                    [str(converter), "--width", "128", "--height", "96",
                     "--output", str(expected_path), str(static_path)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                width, height, oracle = decode_png(expected_path)
                complete = matrix_compose(
                    matrix_compose(parse_transform_list(parent), placement),
                    expected_target,
                )
                center = (math.floor(complete[4]), math.floor(complete[5]))
                offset = (center[1] * width + center[0]) * 4
                render = actual_path.read_bytes()
                self.assertEqual(render[offset:offset + 4], oracle[offset:offset + 4],
                                 f"animateMotion through {mode}")
                self.assertEqual(oracle[offset:offset + 4], BLUE,
                                 f"librsvg static {mode} center is uncovered")

    def test_animated_transform_and_geometry_refresh_clip_owners(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        with tempfile.TemporaryDirectory(prefix="anim-clips-ancestor-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            cases = (
                (
                    "<defs><clipPath id='c'><rect x='-10' y='-3' width='20' height='6'/>"
                    "</clipPath></defs>",
                    "<g transform='translate(58 42)'><g clip-path='url(#c)'>"
                    "<animateTransform attributeName='transform' type='rotate' "
                    "from='0' to='90' dur='2s' fill='freeze'/>"
                    "<rect x='-24' y='-12' width='48' height='24' fill='#4f7cff'/>"
                    "</g></g>",
                    "<g transform='translate(58 42)'><g transform='rotate(45)' "
                    "clip-path='url(#c)'><rect x='-24' y='-12' width='48' height='24' "
                    "fill='#4f7cff'/></g></g>",
                    ((58, 42, BLUE), (64, 42, bytes((16, 21, 28, 255)))),
                ),
                (
                    "<defs><clipPath id='c' clipPathUnits='objectBoundingBox'>"
                    "<rect x='0' y='0' width='.5' height='1'/></clipPath></defs>",
                    "<g transform='translate(58 42) rotate(10) scale(.9 1.1)'>"
                    "<rect x='-20' y='-10' width='40' height='20' fill='#4f7cff' "
                    "clip-path='url(#c)'><animate attributeName='width' from='40' "
                    "to='20' dur='2s' fill='freeze'/></rect></g>",
                    "<g transform='translate(58 42) rotate(10) scale(.9 1.1)'>"
                    "<rect x='-20' y='-10' width='30' height='20' fill='#4f7cff' "
                    "clip-path='url(#c)'/></g>",
                    ((48, 40, BLUE), (56, 41, bytes((16, 21, 28, 255))),
                     (60, 40, bytes((16, 21, 28, 255)))),
                ),
            )
            for case_index, (definitions, animated_body, static_body, probes) in enumerate(cases):
                animated = (
                    "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
                    "<rect width='128' height='96' fill='#10151c'/>" + definitions +
                    animated_body + "</svg>"
                )
                static = (
                    "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
                    "<rect width='128' height='96' fill='#10151c'/>" + definitions +
                    static_body + "</svg>"
                )
                animated_path.write_text(animated, encoding="utf-8")
                static_path.write_text(static, encoding="utf-8")
                subprocess.run(
                    [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                subprocess.run(
                    [str(converter), "--width", "128", "--height", "96",
                     "--output", str(expected_path), str(static_path)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                width, height, oracle = decode_png(expected_path)
                self.assertEqual((width, height), (128, 96))
                render = actual_path.read_bytes()
                self.assertEqual(len(render), width * height * 4)
                for x, y, expected in probes:
                    offset = (y * width + x) * 4
                    self.assertEqual(
                        render[offset:offset + 4], oracle[offset:offset + 4],
                        f"clip case {case_index} differs from static librsvg at ({x},{y})",
                    )
                    self.assertEqual(
                        oracle[offset:offset + 4], expected,
                        f"clip case {case_index} reference probe ({x},{y})",
                    )

    def test_multiple_transform_tracks_compose_in_document_order(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        first = matrix_for("translate", (4.0, 0.0))
        second = matrix_for("rotate", (45.0,))
        combined = matrix_compose(first, second)
        animated = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
            "<g transform='translate(58 42)'><rect x='-12' y='-8' width='24' "
            "height='16' fill='#4f7cff' transform='translate(10 0)'>"
            "<animateTransform attributeName='transform' type='translate' "
            "from='0 0' to='8 0' dur='2s' fill='freeze'/>"
            "<animateTransform attributeName='transform' type='rotate' "
            "from='0' to='90' dur='2s' fill='freeze'/>"
            "</rect></g></svg>"
        )
        static = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
            "<g transform='translate(58 42)'><rect x='-12' y='-8' width='24' "
            f"height='16' fill='#4f7cff' transform='{matrix_attribute(combined)}'/>"
            "</g></svg>"
        )
        with tempfile.TemporaryDirectory(prefix="anim-multiple-transform-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            animated_path.write_text(animated, encoding="utf-8")
            static_path.write_text(static, encoding="utf-8")
            subprocess.run(
                [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            subprocess.run(
                [str(converter), "--width", "128", "--height", "96",
                 "--output", str(expected_path), str(static_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            width, height, oracle = decode_png(expected_path)
            render = actual_path.read_bytes()
            self.assertEqual((width, height), (128, 96))
            center = (62, 42)
            offset = (center[1] * width + center[0]) * 4
            self.assertEqual(render[offset:offset + 4], oracle[offset:offset + 4])
            self.assertEqual(oracle[offset:offset + 4], BLUE)

    def test_animated_geometry_refreshes_object_box_gradient_bounds(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        document_start = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='64' height='48'>"
            "<defs><linearGradient id='g'><stop offset='0' stop-color='#ff0000'/>"
            "<stop offset='1' stop-color='#0000ff'/></linearGradient></defs>"
        )
        animated = (
            document_start + "<rect x='8' y='8' width='20' height='24' fill='url(#g)'>"
            "<animate attributeName='width' from='20' to='40' dur='2s' "
            "fill='freeze'/></rect></svg>"
        )
        static = (
            document_start + "<rect x='8' y='8' width='30' height='24' "
            "fill='url(#g)'/></svg>"
        )
        with tempfile.TemporaryDirectory(prefix="anim-gradient-bounds-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            animated_path.write_text(animated, encoding="utf-8")
            static_path.write_text(static, encoding="utf-8")
            subprocess.run(
                [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            subprocess.run(
                [str(converter), "--width", "64", "--height", "48",
                 "--output", str(expected_path), str(static_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            width, height, oracle = decode_png(expected_path)
            render = actual_path.read_bytes()
            self.assertEqual((width, height), (64, 48))
            offset = (20 * width + 23) * 4
            actual_pixel = render[offset:offset + 4]
            expected_pixel = oracle[offset:offset + 4]
            self.assertLessEqual(
                max(abs(a - b) for a, b in zip(actual_pixel, expected_pixel)), 2,
                f"object-box gradient differs: {actual_pixel!r} vs {expected_pixel!r}",
            )

    def test_animated_filtered_group_returns_stable_unsupported(self) -> None:
        sources = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'>"
            "<defs><filter id='f'><feGaussianBlur stdDeviation='1'/></filter></defs>"
            "<g filter='url(#f)' transform='translate(4 4)'>"
            "<animateTransform attributeName='transform' type='rotate' from='0' "
            "to='20' dur='1s' fill='freeze'/>"
            "<rect width='16' height='16' fill='#4f7cff'/></g></svg>",
            "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'>"
            "<g><animate attributeName='opacity' from='1' to='.5' dur='1s' "
            "fill='freeze'/><rect width='16' height='16' fill='#4f7cff'/></g></svg>",
            "<svg xmlns='http://www.w3.org/2000/svg' width='32' height='32'>"
            "<defs><pattern id='p' width='4' height='4'><rect width='4' "
            "height='4' fill='red'/></pattern></defs><rect width='16' height='16' "
            "fill='url(#p)'><animate attributeName='width' from='16' to='8' "
            "dur='1s' fill='freeze'/></rect></svg>",
        )
        with tempfile.TemporaryDirectory(prefix="anim-filter-unsupported-") as temporary:
            for index, source in enumerate(sources):
                path = Path(temporary) / f"unsupported-{index}.svg"
                path.write_text(source, encoding="utf-8")
                result = subprocess.run(
                    [str(ANIMATED_RENDER), str(path), "1",
                     str(path.with_suffix(".rgba"))],
                    cwd=ROOT, capture_output=True, text=True, timeout=20,
                )
                self.assertNotEqual(result.returncode, 0)
                self.assertRegex(
                    result.stderr, r"^unsupported at byte [0-9]+$",
                    f"unsupported case {index}: {result.returncode} {result.stderr!r}",
                )

    def test_animated_use_host_transform_preserves_x_y_placement(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        common = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='128' height='96'>"
            "<rect width='128' height='96' fill='#10151c'/>"
            "<g transform='translate(58 42)'><defs><g id='asset' "
            "transform='translate(1 2)'><rect x='-8' y='-6' width='16' "
            "height='12' fill='#4f7cff'/></g></defs>"
        )
        animated = (
            common + "<use href='#asset' x='2' y='1'>"
            "<animateTransform attributeName='transform' type='rotate' "
            "from='0' to='90' dur='2s' fill='freeze'/>"
            "</use></g></svg>"
        )
        static = (
            common + "<use href='#asset' x='2' y='1' transform='rotate(45)'/>"
            "</g></svg>"
        )
        with tempfile.TemporaryDirectory(prefix="anim-use-host-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            animated_path.write_text(animated, encoding="utf-8")
            static_path.write_text(static, encoding="utf-8")
            subprocess.run(
                [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            subprocess.run(
                [str(converter), "--width", "128", "--height", "96",
                 "--output", str(expected_path), str(static_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            width, height, oracle = decode_png(expected_path)
            render = actual_path.read_bytes()
            self.assertEqual((width, height), (128, 96))
            x, y = 52, 46
            offset = (y * width + x) * 4
            self.assertEqual(render[offset:offset + 4], oracle[offset:offset + 4])
            self.assertEqual(oracle[offset:offset + 4], BLUE)

    def test_animated_child_transform_follows_ancestor_translation(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        animated = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='200' height='120'>"
            "<rect width='200' height='120' fill='#10151c'/>"
            "<g transform='translate(60 60)'>"
            "<rect x='-30' y='-30' width='60' height='60' fill='#4f7cff'>"
            "<animateTransform attributeName='transform' type='rotate' from='0' "
            "to='360' dur='4s' repeatCount='indefinite'/>"
            "</rect></g></svg>"
        )
        expected = (
            "<svg xmlns='http://www.w3.org/2000/svg' width='200' height='120'>"
            "<rect width='200' height='120' fill='#10151c'/>"
            "<g transform='translate(60 60)'>"
            "<rect x='-30' y='-30' width='60' height='60' fill='#4f7cff' "
            "transform='rotate(90)'/></g></svg>"
        )
        with tempfile.TemporaryDirectory(prefix="anim-ancestor-") as temporary:
            directory = Path(temporary)
            animated_path = directory / "animated.svg"
            static_path = directory / "static.svg"
            actual_path = directory / "actual.rgba"
            expected_path = directory / "expected.png"
            animated_path.write_text(animated, encoding="utf-8")
            static_path.write_text(expected, encoding="utf-8")
            subprocess.run(
                [str(ANIMATED_RENDER), str(animated_path), "1", str(actual_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            subprocess.run(
                [str(converter), "--width", "200", "--height", "120",
                 "--output", str(expected_path), str(static_path)],
                cwd=ROOT, check=True, capture_output=True, timeout=20,
            )
            width, height, oracle = decode_png(expected_path)
            self.assertEqual((width, height), (200, 120))
            render = actual_path.read_bytes()
            self.assertEqual(len(render), 200 * 120 * 4)
            for x, y in ((60, 60), (45, 45), (75, 75)):
                offset = (y * width + x) * 4
                self.assertEqual(
                    render[offset:offset + 4], oracle[offset:offset + 4],
                    f"ancestor composition differs at ({x},{y})",
                )

    def test_sampled_animation_pixels_match_librsvg_static_values(self) -> None:
        converter = shutil.which("rsvg-convert")
        self.assertIsNotNone(converter, "librsvg oracle must be installed for this lane")
        with tempfile.TemporaryDirectory(prefix="anim-rsvg-") as temporary:
            directory = Path(temporary)
            for seconds in (0.0, 0.5, 1.0, 1.5):
                x = 1.0 + (39.0 - 1.0) * ((seconds % 2.0) / 2.0)
                static = directory / "static.svg"
                static.write_text(
                    f'<svg xmlns="http://www.w3.org/2000/svg" width="48" height="16">'
                    f'<rect x="{x:.9g}" y="4" width="8" height="8" fill="#2a9d8f"/>'
                    f'</svg>', encoding="utf-8"
                )
                actual = directory / "actual.rgba"
                expected = directory / "expected.png"
                subprocess.run(
                    [str(ANIMATED_RENDER), str(FIXTURE), f"{seconds:.17g}", str(actual)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                subprocess.run(
                    [str(converter), "--width", "48", "--height", "16", "--output", str(expected), str(static)],
                    cwd=ROOT, check=True, capture_output=True, timeout=20,
                )
                width, height, oracle = decode_png(expected)
                self.assertEqual((width, height), (48, 16))
                render = actual.read_bytes()
                self.assertEqual(len(render), 48 * 16 * 4)
                x_probe = min(46, int(x) + 3)
                offset = (8 * width + x_probe) * 4
                self.assertEqual(render[offset:offset + 4], oracle[offset:offset + 4],
                                 f"pixel differs at t={seconds}")


if __name__ == "__main__":
    unittest.main()
