"""Seeded paint and clip document generator for renderer differential tests."""

def random_paint_clip_document(rng, index):
    width, height = 40, 32
    fraction = (0.25, 0.5, 0.75)[index % 3]
    gradient_kind = "linearGradient" if index % 2 == 0 else "radialGradient"
    gradient_units = "objectBoundingBox" if (index // 2) % 2 == 0 else "userSpaceOnUse"
    outer_units = "objectBoundingBox" if (index // 4) % 2 == 0 else "userSpaceOnUse"
    inner_units = "userSpaceOnUse" if outer_units == "objectBoundingBox" else "objectBoundingBox"
    if gradient_kind == "linearGradient":
        coords = (' x1="0" y1="0" x2="0.22" y2="0"' if gradient_units == "objectBoundingBox"
                  else ' x1="0" y1="0" x2="8" y2="0"')
    else:
        coords = (' cx="0.5" cy="0.5" r="0.25"' if gradient_units == "objectBoundingBox"
                  else ' cx="20" cy="16" r="8"')
    opacity_a = rng.choice(("0.25", "0.45", "0.7"))
    opacity_b = rng.choice(("0.35", "0.6", "0.85"))
    gradient = (f'<{gradient_kind} id="paint" gradientUnits="{gradient_units}"{coords}>'
                f'<stop offset="0" stop-color="#e02020" stop-opacity="{opacity_a}"/>'
                '<stop offset="0.55" stop-color="#20a060" stop-opacity="0.8"/>'
                f'<stop offset="1" stop-color="#2040e0" stop-opacity="{opacity_b}"/>'
                f'</{gradient_kind}>')
    clip_rotation = rng.uniform(-18.0, 18.0)
    clip_scale_x, clip_scale_y = rng.uniform(0.9, 1.1), rng.uniform(0.9, 1.1)
    clip_transform = (f' transform="rotate({clip_rotation:.3f} 20 16) '
                      f'scale({clip_scale_x:.3f} {clip_scale_y:.3f})"')
    if outer_units == "objectBoundingBox":
        outer_clip = ('<clipPath id="outer" clipPathUnits="objectBoundingBox">'
                      f'<rect x="0.05" y="0.1" width="0.72" height="0.8"{clip_transform}/>'
                      '<circle cx="0.78" cy="0.52" r="0.2"/></clipPath>')
    else:
        edge = rng.randrange(14, 25) + fraction
        outer_clip = (f'<clipPath id="outer" clipPathUnits="userSpaceOnUse">'
                      f'<rect x="2.5" y="1.5" width="{edge:.2f}" height="28.5"{clip_transform}/>'
                      '<circle cx="31" cy="16" r="7"/></clipPath>')
    if inner_units == "objectBoundingBox":
        inner_clip = ('<clipPath id="inner" clipPathUnits="objectBoundingBox">'
                      f'<rect x="0.22" y="0.08" width="0.72" height="0.84"{clip_transform}/>'
                      '</clipPath>')
    else:
        inner_edge = rng.randrange(25, 36) + fraction
        inner_clip = (f'<clipPath id="inner" clipPathUnits="userSpaceOnUse">'
                      f'<rect x="1.5" y="1.5" width="{inner_edge:.2f}" height="28.5"{clip_transform}/>'
                      '</clipPath>')
    fractional_clip = (f'<clipPath id="fractional" clipPathUnits="userSpaceOnUse">'
                       f'<rect x="0" y="0" width="{22.0 + fraction:.2f}" '
                       'height="32"/></clipPath>')
    x, y = rng.uniform(1.0, 7.0), rng.uniform(1.0, 5.0)
    rect_w, rect_h = rng.uniform(25.0, 31.0), rng.uniform(20.0, 25.0)
    translate_x, translate_y = rng.uniform(-1.5, 1.5), rng.uniform(-1.0, 1.0)
    angle = rng.uniform(-18.0, 18.0)
    scale_x, scale_y = rng.uniform(0.85, 1.15), rng.uniform(0.85, 1.15)
    defs = f'<defs>{gradient}{outer_clip}{inner_clip}{fractional_clip}</defs>'
    underlay = '<rect x="4" y="4" width="28" height="20" fill="#503080"/>'
    content = ('<g clip-path="url(#fractional)">'
               f'<g transform="translate({translate_x:.3f} {translate_y:.3f}) '
               f'rotate({angle:.3f}) scale({scale_x:.3f} {scale_y:.3f}) '
               'clip-path="url(#outer)"><g clip-path="url(#inner)">'
               f'<rect x="{x:.3f}" y="{y:.3f}" width="{rect_w:.3f}" '
               f'height="{rect_h:.3f}" fill="url(#paint)"/>'
               '</g></g></g>')
    return (f'<svg width="{width}" height="{height}" viewBox="0 0 {width} {height}">'
            f'{defs}{underlay}{content}</svg>')
