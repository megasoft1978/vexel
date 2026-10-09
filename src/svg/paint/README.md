# SVG paint servers

This module resolves only bounded local linear and radial gradients used by a
shape's computed `fill`. It owns the parsed stop array through the decoded
shape document and samples pad-spread interpolation in the scalar rasterizer.

The current subset allows 2–256 stops per gradient and 65,536 total copied
stops. Colors are literal supported names or hex values; an omitted
`stop-color` defaults to SVG's initial black value. Offsets and opacity are
finite, normalized, and monotone. Object-bounding-box and user-space units
are recognized. User-space coordinates resolve percentages against the root
viewBox and use the SVG defaults for linear endpoints and radial center, radius,
and focal point. Radial gradients with a non-centered explicit focal point,
gradient transforms, inherited paint servers, unsupported units, external
references, and unsupported stop syntax fail with a stable unsupported error.

No external resource is loaded. `svg_shape_document_destroy` releases the
paint-server stops.
