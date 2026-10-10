#!/usr/bin/env python3
"""Generate the checked, project-authored real-world SVG robustness corpus."""
import hashlib
import json
import math
import random
from pathlib import Path


ROOT = Path(__file__).resolve().parent
FIXTURES = ROOT / "fixtures"


def document(title, width, height, body, *, defs="", styles="", namespaces="",
             metadata="", root_attrs=""):
    return (
        '<svg xmlns="http://www.w3.org/2000/svg" '
        f'xmlns:xlink="http://www.w3.org/1999/xlink" {namespaces} '
        f'width="{width}" height="{height}" viewBox="0 0 {width} {height}" '
        f'{root_attrs}>'
        f'<title>{title}</title>{metadata}{styles}{defs}{body}</svg>\n'
    )


FEATHER = {
    "activity": '<path d="M8 34h12l7-19 12 39 7-20h10"/>',
    "archive": '<path d="M12 13h40v10H12zM16 23v27h32V23M25 34h14"/>',
    "bell": '<path d="M18 42h28l-4-5V27a10 10 0 0 0-20 0v10zM27 49a5 5 0 0 0 10 0"/>',
    "calendar": '<rect x="12" y="15" width="40" height="36" rx="4"/><path d="M21 10v11M43 10v11M12 26h40"/>',
    "camera": '<path d="M11 22h9l4-6h16l4 6h9v29H11z"/><circle cx="32" cy="36" r="9"/>',
    "check-circle": '<circle cx="32" cy="32" r="20"/><path d="m21 32 8 8 15-17"/>',
    "clock": '<circle cx="32" cy="32" r="21"/><path d="M32 19v14l10 6"/>',
    "cloud-upload": '<path d="M20 43h-4a10 10 0 0 1 1-20 16 16 0 0 1 30-3 11 11 0 0 1 1 23h-4M25 34l7-7 7 7M32 27v23"/>',
    "download": '<path d="M32 12v28m-11-11 11 11 11-11M14 51h36"/>',
    "eye": '<path d="M7 32s9-15 25-15 25 15 25 15-9 15-25 15S7 32 7 32z"/><circle cx="32" cy="32" r="7"/>',
    "file-text": '<path d="M17 9h19l11 11v35H17zM36 9v12h11M24 32h16M24 40h16M24 48h11"/>',
    "folder": '<path d="M8 18h20l5 6h23v27H8z"/>',
    "heart": '<path d="M32 51S10 38 10 24a11 11 0 0 1 20-7l2 3 2-3a11 11 0 0 1 20 7C54 38 32 51 32 51z"/>',
    "lock": '<rect x="13" y="27" width="38" height="27" rx="4"/><path d="M21 27v-8a11 11 0 0 1 22 0v8M32 37v7"/>',
    "mail": '<rect x="9" y="16" width="46" height="32" rx="4"/><path d="m11 20 21 16 21-16"/>',
    "map-pin": '<path d="M49 27c0 13-17 29-17 29S15 40 15 27a17 17 0 1 1 34 0z"/><circle cx="32" cy="27" r="5"/>',
    "search": '<circle cx="28" cy="28" r="16"/><path d="m40 40 12 12"/>',
    "settings": '<circle cx="32" cy="32" r="8"/><path d="M32 10v7m0 30v7M10 32h7m30 0h7M16 16l5 5m22 22 5 5m0-32-5 5m-22 22-5 5"/>',
    "shopping-cart": '<circle cx="24" cy="49" r="3"/><circle cx="46" cy="49" r="3"/><path d="M9 13h7l7 28h25l7-20H19"/>',
    "user": '<circle cx="32" cy="22" r="10"/><path d="M12 54a20 20 0 0 1 40 0"/>',
}

MATERIAL = {
    "account": '<path d="M32 32a12 12 0 1 0 0-24 12 12 0 0 0 0 24zm0 6C18 38 8 45 8 54v4h48v-4c0-9-10-16-24-16z"/>',
    "bolt": '<path d="M36 4 12 35h16l-2 25 22-34H32z"/>',
    "bookmark": '<path d="M16 8h32v50L32 46 16 58z"/>',
    "chat": '<path d="M8 10h48v34H29L15 56v-12H8z"/>',
    "dashboard": '<path d="M8 8h21v21H8zm27 0h21v12H35zM35 26h21v30H35zM8 35h21v21H8z"/>',
    "edit": '<path d="m10 44-2 14 14-2 31-31-12-12zM47 8l9 9-5 5-9-9z"/>',
    "favorite": '<path d="M32 57 8 34C-4 21 15 3 28 17l4 4 4-4c13-14 32 4 20 17z"/>',
    "home": '<path d="m5 29 27-23 27 23-4 5-5-4v26H35V39H26v17H14V30l-5 4z"/>',
    "layers": '<path d="m32 5 27 14-27 14L5 19zm-22 24 22 12 22-12 5 5-27 15L5 34zm0 15 22 12 22-12 5 5-27 15L5 49z"/>',
    "notifications": '<path d="M32 6a4 4 0 0 0-4 4v2C17 14 12 22 12 33v12L7 51v3h50v-3l-5-6V33c0-11-5-19-16-21v-2a4 4 0 0 0-4-4zm-7 51h14a7 7 0 0 1-14 0z"/>',
    "place": '<path d="M32 4a21 21 0 0 0-21 21c0 16 21 35 21 35s21-19 21-35A21 21 0 0 0 32 4zm0 29a8 8 0 1 1 0-16 8 8 0 0 1 0 16z"/>',
    "verified": '<path d="m32 4 7 5 9-1 3 9 7 6-4 9 1 9-9 4-5 8-9-3-9 3-5-8-9-4 1-9-4-9 7-6 3-9 9 1zm-4 39 17-18-4-4-13 13-7-7-4 4z"/>',
}


def write_fixture(category, filename, source, supported=True, note="selected SVG subset"):
    path = FIXTURES / category / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    data = source.encode("utf-8")
    path.write_bytes(data)
    return {"file": str(path.relative_to(ROOT)), "category": category,
            "supported": supported, "diagnostic": None if supported else note,
            "width": int(source.split('width="', 1)[1].split('"', 1)[0]),
            "height": int(source.split('height="', 1)[1].split('"', 1)[0]),
            "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest()}


def build_icons(entries):
    stroke_style = ('<style>.icon{fill:none;stroke:#334155;stroke-width:3;'
                    'stroke-linecap:round;stroke-linejoin:round}</style>')
    for name, art in FEATHER.items():
        body = f'<g class="icon">{art}</g>'
        source = document(f"Outline interface icon: {name}", 64, 64, body,
                          styles=stroke_style)
        entries.append(write_fixture("feather", f"{name}.svg", source))
    for name, art in MATERIAL.items():
        fill = "#2563eb" if name in ("home", "place", "verified") else "#475569"
        path_data = art[art.find('d="') + 3:art.rfind('"')]
        body = f'<path d="{path_data}" fill="{fill}"/>'
        source = document(f"Filled interface icon: {name}", 64, 64, body)
        entries.append(write_fixture("material", f"{name}.svg", source))


def build_inkscape(entries):
    namespaces = ('xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape" '
                  'xmlns:sodipodi="http://sodipodi.sourceforge.net/DTD/sodipodi-0.dtd" '
                  'xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#" '
                  'xmlns:cc="http://creativecommons.org/ns#" '
                  'xmlns:dc="http://purl.org/dc/elements/1.1/"')
    for index in range(8):
        name = f"inkscape-page-{index + 1:02d}"
        metadata = ('<metadata><rdf:RDF><cc:Work rdf:about="">'
                    f'<dc:title>{name.replace("-", " ")}</dc:title>'
                    '<dc:creator>Vexel real-world fixture</dc:creator>'
                    '</cc:Work></rdf:RDF></metadata>')
        namedview = ('<sodipodi:namedview id="namedview1" pagecolor="#ffffff" '
                     'borderopacity="1.0" inkscape:document-units="px"/>')
        art = (f'<g inkscape:label="Layer {index + 1}" '
               f'transform="translate({index % 4} {index % 3}) rotate({index - 3} 48 48)">'
               f'<path d="M18 70 C18 30 78 30 78 70" fill="none" stroke="#0f766e" '
               'stroke-width="5" stroke-linecap="round"/>'
               f'<circle cx="48" cy="40" r="{12 + index}" fill="#f59e0b"/>'
               '</g>')
        source = document(name, 96, 96, namedview + art, namespaces=namespaces,
                          metadata=metadata,
                          root_attrs=f'inkscape:version="1.3" sodipodi:docname="{name}.svg" ')
        entries.append(write_fixture("inkscape", f"{name}.svg", source))


def build_illustrator(entries):
    namespaces = 'xmlns:ai="http://ns.adobe.com/AdobeIllustrator/10.0/" '
    for index in range(6):
        name = f"illustrator-export-{index + 1:02d}"
        accent = ("#2563eb", "#db2777", "#0891b2", "#ea580c", "#7c3aed", "#16a34a")[index]
        defs = ('<defs><linearGradient id="brand" x1="0" y1="0" x2="1" y2="1">'
                f'<stop offset="0" stop-color="{accent}"/><stop offset="1" stop-color="#fbbf24"/>'
                '</linearGradient><clipPath id="artboard"><rect x="8" y="8" width="80" height="80" rx="12"/></clipPath>'
                '<style>.plate{fill:#f8fafc}.stroke{fill:none;stroke:#0f172a;stroke-width:2}</style></defs>')
        art = ('<g id="Layer_1" ai:artboard="0" clip-path="url(#artboard)">'
               '<rect class="plate" x="8" y="8" width="80" height="80" rx="12"/>'
               '<path d="M20 58 C30 18 65 18 76 58 C64 79 32 79 20 58Z" fill="url(#brand)"/>'
               '<path class="stroke" d="M28 48 Q48 28 68 48 M31 60 Q48 70 65 60"/>'
               '</g>')
        source = document(f"Illustrator style export {index + 1}", 96, 96, art,
                          defs=defs, namespaces=namespaces,
                          root_attrs='xml:space="preserve" version="1.1" ')
        entries.append(write_fixture("illustrator", f"{name}.svg", source))


def build_charts(entries):
    for chart_index in range(5):
        width, height = 320, 180
        color_sets = ("#2563eb", "#16a34a", "#ea580c", "#9333ea", "#0891b2")
        grid = ''.join(
            f'<path d="M36 {28 + y * 27}H306" stroke="#cbd5e1" stroke-width="1"/>'
            for y in range(5))
        bars = []
        for series in range(3):
            color = color_sets[(chart_index + series) % len(color_sets)]
            for item in range(24):
                x = 42 + item * 10 + series * 3
                value = 12 + ((item * (chart_index + 3) + series * 17) % 76)
                y = 151 - value
                bars.append(f'<path d="M{x} 151V{y}h2v{value}z" fill="{color}"/>')
        axes = '<path d="M36 18V151H306" fill="none" stroke="#334155" stroke-width="2"/>'
        source = document(f"Sales chart with 72 bars: {chart_index + 1}", width, height,
                          grid + ''.join(bars) + axes)
        entries.append(write_fixture("charts", f"bar-chart-{chart_index + 1:02d}.svg", source))


def build_logos(entries):
    for index in range(4):
        color = ("#0f766e", "#1d4ed8", "#be123c", "#6d28d9")[index]
        curves = []
        for segment in range(18):
            x = 12 + segment * 4
            phase = (segment + index * 0.4) * 0.45
            start_y = 40 + math.sin(phase) * 10
            end_y = 40 + math.sin(phase + 0.45) * 10
            curves.append(f'C{x + 1} {start_y - 3:.2f} '
                          f'{x + 3} {end_y + 3:.2f} {x + 4} {end_y:.2f}')
        first_y = 40 + math.sin(index * 0.4 * 0.45) * 10
        curve_data = ' '.join(curves)
        emblem = (f'<path d="M12 {first_y:.2f} {curve_data}" fill="none" '
                  f'stroke="{color}" stroke-width="4" stroke-linecap="round"/>')
        nodes = ''.join(f'<circle cx="{16 + node * 12}" cy="40" r="3" fill="{color}"/>'
                        for node in range(6))
        defs = ('<defs><linearGradient id="brand"><stop stop-color="'
                f'{color}"/><stop offset="1" stop-color="#fbbf24"/></linearGradient></defs>')
        source = document(f"Curved wordmark study {index + 1}", 96, 80,
                          f'<g transform="translate(0 {index % 2})">{emblem}{nodes}</g>',
                          defs=defs)
        entries.append(write_fixture("logos", f"curve-mark-{index + 1:02d}.svg", source))


def build_special(entries):
    special = [
        ("dashed-cubic", '<path d="M8 48 C22 8 40 8 52 48 S78 88 88 48" fill="none" stroke="#166534" stroke-width="5" stroke-linecap="round" stroke-dasharray="12 6"/>'),
        ("dotted-circuit", '<path d="M12 20H76V42H28V64H84" fill="none" stroke="#7c3aed" stroke-width="3" stroke-linecap="round" stroke-dasharray="1 6"/>'),
        ("arc-and-evenodd", '<path fill="#0f766e" fill-rule="evenodd" d="M12 48 A36 36 0 1 0 84 48 A36 36 0 1 0 12 48 M30 48 A18 18 0 1 1 66 48 A18 18 0 1 1 30 48Z"/>'),
        ("nested-clip-gradient", '<g clip-path="url(#window)" transform="translate(6 3) rotate(5 48 48)"><path d="M10 70 C20 10 72 10 86 70L78 88H18Z" fill="url(#sunset)"/><path d="M20 52H76M24 62H72" stroke="#fff" stroke-width="3" stroke-dasharray="5 3"/></g>'),
    ]
    for name, art in special:
        defs = ('<defs><clipPath id="window"><rect x="10" y="10" width="76" height="76" rx="8"/></clipPath>'
                '<linearGradient id="sunset"><stop stop-color="#fb7185"/><stop offset="1" stop-color="#fbbf24"/></linearGradient></defs>')
        source = document(f"SVG robustness sample: {name}", 96, 96, art, defs=defs)
        entries.append(write_fixture("special", f"{name}.svg", source))


def build_w5(entries):
    samples = [
        ("mask-alpha", '<rect x="8" y="8" width="80" height="48" fill="#2563eb" mask="url(#mask)"/>',
         '<mask id="mask" mask-type="alpha" maskUnits="userSpaceOnUse" x="0" y="0" width="96" height="64"><circle cx="48" cy="32" r="24" fill="#fff" fill-opacity=".8"/></mask>'),
        ("mask-luminance", '<rect x="8" y="8" width="80" height="48" fill="#be123c" mask="url(#mask)"/>',
         '<mask id="mask" mask-type="luminance"><rect width="1" height="1" fill="#fff"/><circle cx=".5" cy=".5" r=".38" fill="#777"/></mask>'),
        ("mask-object-bounds", '<rect x="16" y="12" width="64" height="40" fill="#0891b2" mask="url(#mask)"/>',
         '<mask id="mask" maskUnits="objectBoundingBox" maskContentUnits="objectBoundingBox" x="0" y="0" width="1" height="1"><rect x=".1" y=".1" width=".8" height=".8" fill="#fff"/></mask>'),
        ("mask-nested", '<rect x="8" y="8" width="80" height="48" fill="#16a34a" mask="url(#outer)"/>',
         '<mask id="inner" mask-type="alpha"><circle cx=".5" cy=".5" r=".48" fill="#fff"/></mask><mask id="outer" mask-type="alpha"><rect width="1" height="1" fill="#fff" mask="url(#inner)"/></mask>'),
        ("pattern-user-space", '<rect width="96" height="64" fill="url(#tile)"/>',
         '<pattern id="tile" patternUnits="userSpaceOnUse" width="12" height="12"><rect width="6" height="6" fill="#2563eb"/><circle cx="9" cy="9" r="2" fill="#f59e0b"/></pattern>'),
        ("pattern-object-bounds", '<rect x="8" y="8" width="80" height="48" fill="url(#tile)"/>',
         '<pattern id="tile" patternUnits="objectBoundingBox" patternContentUnits="objectBoundingBox" width=".25" height=".25"><rect width=".5" height=".5" fill="#7c3aed"/><circle cx=".75" cy=".75" r=".12" fill="#fbbf24"/></pattern>'),
        ("pattern-transform", '<rect x="4" y="4" width="88" height="56" fill="url(#tile)"/>',
         '<pattern id="tile" patternUnits="userSpaceOnUse" width="16" height="16" patternTransform="rotate(20)"><path d="M0 0V16" stroke="#0f766e" stroke-width="4"/></pattern>'),
        ("pattern-viewbox", '<rect width="96" height="64" fill="url(#tile)"/>',
         '<pattern id="tile" patternUnits="userSpaceOnUse" width="16" height="12" viewBox="0 0 8 6"><rect width="4" height="3" fill="#ea580c"/><circle cx="6" cy="4.5" r="1.2" fill="#0f172a"/></pattern>'),
        ("pattern-stroke", '<path d="M10 32H86" fill="none" stroke="url(#tile)" stroke-width="16"/>',
         '<pattern id="tile" patternUnits="userSpaceOnUse" width="8" height="8"><rect width="4" height="8" fill="#db2777"/><rect x="4" width="4" height="8" fill="#fbcfe8"/></pattern>'),
        ("marker-path", '<path d="M12 48L48 16L84 48" fill="none" stroke="#334155" stroke-width="4" marker-start="url(#arrow)" marker-mid="url(#arrow)" marker-end="url(#arrow)"/>',
         '<marker id="arrow" markerWidth="8" markerHeight="8" refX="4" refY="4" orient="auto" markerUnits="strokeWidth"><path d="M0 0L8 4L0 8Z" fill="#f97316" stroke="none"/></marker>'),
        ("marker-shapes", '<line x1="10" y1="16" x2="86" y2="16" marker-start="url(#dot)" marker-end="url(#dot)" stroke="#0f766e"/><polyline points="10,32 48,48 86,32" marker-start="url(#dot)" marker-mid="url(#dot)" marker-end="url(#dot)" fill="none" stroke="#0f766e" stroke-width="3"/><polygon points="10,56 48,40 86,56" marker-start="url(#dot)" marker-mid="url(#dot)" marker-end="url(#dot)" fill="none" stroke="#0f766e" stroke-width="3"/>',
         '<marker id="dot" markerWidth="6" markerHeight="6" refX="3" refY="3" orient="auto"><circle cx="3" cy="3" r="2" fill="#7c3aed" stroke="none"/></marker>'),
        ("marker-viewbox", '<path d="M12 48H84" fill="none" stroke="#1d4ed8" stroke-width="3" marker-end="url(#arrow)"/>',
         '<marker id="arrow" markerWidth="16" markerHeight="12" viewBox="0 0 10 10" refX="8" refY="5" orient="auto" preserveAspectRatio="xMidYMid meet"><path d="M0 0L10 5L0 10Z" fill="#dc2626" stroke="none"/></marker>'),
        ("marker-start-reverse", '<path d="M12 32H84" fill="none" stroke="#334155" stroke-width="4" marker-start="url(#arrow)" marker-end="url(#arrow)"/>',
         '<marker id="arrow" markerWidth="8" markerHeight="8" refX="4" refY="4" orient="auto-start-reverse"><path d="M0 0L8 4L0 8Z" fill="#059669" stroke="none"/></marker>'),
        ("filter-blur-offset", '<rect x="20" y="16" width="40" height="32" rx="5" fill="#2563eb" filter="url(#effect)"/>',
         '<filter id="effect"><feGaussianBlur in="SourceGraphic" stdDeviation="1" result="soft"/><feOffset in="soft" dx="5" dy="3" result="shift"/><feBlend in="SourceGraphic" in2="shift" mode="screen"/></filter>'),
        ("filter-flood-composite", '<circle cx="48" cy="32" r="24" fill="#16a34a" filter="url(#effect)"/>',
         '<filter id="effect"><feFlood flood-color="#facc15" result="color"/><feComposite in="color" in2="SourceGraphic" operator="in"/></filter>'),
        ("filter-blend", '<g filter="url(#effect)"><rect x="18" y="14" width="40" height="36" fill="#dc2626"/><circle cx="58" cy="38" r="20" fill="#2563eb"/></g>',
         '<filter id="effect"><feBlend in="SourceGraphic" in2="SourceGraphic" mode="multiply"/></filter>'),
        ("filter-merge", '<circle cx="48" cy="32" r="18" fill="#7c3aed" filter="url(#effect)"/>',
         '<filter id="effect"><feMerge><feMergeNode in="SourceGraphic"/><feMergeNode in="SourceGraphic"/></feMerge></filter>'),
        ("filter-color-matrix", '<rect x="12" y="12" width="72" height="40" rx="8" fill="#0891b2" filter="url(#effect)"/>',
         '<filter id="effect"><feColorMatrix type="hueRotate" values="45"/></filter>'),
        ("filter-drop-shadow", '<path d="M18 48L48 12L78 48Z" fill="#ea580c" filter="url(#effect)"/>',
         '<filter id="effect"><feDropShadow dx="3" dy="4" stdDeviation="2" flood-color="#ea580c" flood-opacity=".65"/></filter>'),
        ("switch-language", '<switch><g systemLanguage="en"><rect x="12" y="12" width="72" height="40" rx="8" fill="#2563eb"/></g><rect x="12" y="12" width="72" height="40" rx="8" fill="#2563eb"/></switch>', ''),
        ("blend-isolation", '<rect x="12" y="14" width="40" height="36" fill="#dc2626"/><g style="mix-blend-mode:multiply;isolation:isolate"><circle cx="52" cy="38" r="22" fill="#2563eb"/><rect x="60" y="12" width="24" height="42" fill="#facc15"/></g>', ''),
        ("paint-order", '<path d="M16 50L48 12L80 50Z" fill="#f97316" stroke="#1e293b" stroke-width="8" paint-order="stroke fill"/>', ''),
        ("non-scaling-stroke", '<path d="M12 16L84 48" fill="none" stroke="#be123c" stroke-width="5" vector-effect="non-scaling-stroke" transform="scale(1.2)"/>', ''),
        ("crisp-edges", '<path d="M8 8H88V56H8Z M24 20H72V44H24Z" fill="#0f766e" fill-rule="evenodd" shape-rendering="crispEdges"/>', ''),
        ("join-fallbacks", '<path d="M16 50L36 14L56 50" fill="none" stroke="none" stroke-width="9" stroke-linejoin="arcs"/><path d="M48 50L68 14L88 50" fill="none" stroke="none" stroke-width="9" stroke-linejoin="miter-clip"/>', ''),
        ("anchor-passthrough", '<a><rect x="12" y="12" width="72" height="40" rx="8" fill="#0891b2"/></a>', ''),
    ]
    for name, body, defs in samples:
        definitions = f"<defs>{defs}</defs>" if defs else ""
        source = document(f"W5 rendering sample: {name}", 96, 64, body,
                          defs=definitions)
        entries.append(write_fixture("w5", f"{name}.svg", source))
    unsupported = document(
        "W5 unsupported filter sample: morphology", 96, 64,
        '<rect x="16" y="12" width="64" height="40" fill="#2563eb" '
        'filter="url(#effect)"/>',
        defs='<defs><filter id="effect"><feMorphology operator="dilate" '
             'radius="1"/></filter></defs>')
    entries.append(write_fixture("w5", "filter-morphology-unsupported.svg",
                                 unsupported, supported=False,
                                 note="unsupported-feMorphology"))


def build_map(entries):
    rng = random.Random(204810)
    palette = ("#dbeafe", "#bfdbfe", "#dcfce7", "#bbf7d0",
               "#fef3c7", "#fde68a", "#e2e8f0", "#cbd5e1")
    polygons = []
    columns, rows = 50, 40
    cell_w, cell_h = 512 / columns, 512 / rows
    for index in range(columns * rows):
        column, row = index % columns, index // columns
        left, top = column * cell_w, row * cell_h
        points = []
        for corner in range(6):
            angle = corner * 60 + rng.uniform(-12, 12)
            radius = rng.uniform(3.3, 5.2)
            import math
            x = left + cell_w * 0.5 + math.cos(math.radians(angle)) * radius
            y = top + cell_h * 0.5 + math.sin(math.radians(angle)) * radius
            points.append((x, y))
        path = 'M' + ' L'.join(f'{x:.2f} {y:.2f}' for x, y in points) + 'Z'
        polygons.append(f'<path d="{path}" fill="{palette[rng.randrange(len(palette))]}"/>')
    source = document("Synthetic map mosaic with 2,000 local polygons", 512, 512,
                      ''.join(polygons))
    entries.append(write_fixture("maps", "map-mosaic-2000.svg", source))


def main():
    entries = []
    build_icons(entries)
    build_inkscape(entries)
    build_illustrator(entries)
    build_charts(entries)
    build_logos(entries)
    build_special(entries)
    build_w5(entries)
    build_map(entries)
    entries.sort(key=lambda item: item["file"])
    expected = 20 + 12 + 8 + 6 + 5 + 4 + 4 + 1 + 27
    if len(entries) != expected:
        raise SystemExit(f"expected {expected} fixtures, generated {len(entries)}")
    (ROOT / "manifest.json").write_text(
        json.dumps({"schema": 1, "source": "project-authored deterministic fixtures",
                    "count": len(entries), "cases": entries}, indent=2) + "\n",
        encoding="utf-8")
    print(f"generated {len(entries)} SVG fixtures under {FIXTURES}")


if __name__ == "__main__":
    main()
