# SVG paint servers

This module resolves only bounded local linear and radial gradients used by a
shape's computed `fill`. It owns the parsed stop array through the decoded
shape document and samples pad-spread interpolation in the scalar rasterizer.

The current subset allows 2–256 stops per gradient and 65,536 total copied
stops. Colors are literal supported names or hex values. Offsets and opacity
are finite, normalized, and monotone. Object-bounding-box and user-space units
are recognized; radial gradients currently require a centered focal point.
Gradient transforms, inherited paint servers, unsupported units, external
references, and unsupported stop syntax fail with a stable unsupported error.

No external resource is loaded. `svg_shape_document_destroy` releases the
paint-server stops.
