# SVG paint servers

This module resolves only bounded local linear and radial gradients used by a
shape's computed `fill`. It owns the parsed stop array through the decoded
shape document and samples pad-spread interpolation in the scalar rasterizer.
Sampling maps root-space pixel coordinates back through the referencing
element's current inverse transform, including ancestor and `<use>` transforms.
`userSpaceOnUse` therefore follows the referencing element's user space;
`objectBoundingBox` is measured from its untransformed local geometry before
the element transform is applied.

The current subset allows 2–256 stops per gradient and 65,536 total copied
stops. Colors are literal supported names or hex values; an omitted
`stop-color` defaults to SVG's initial black value. `stop-color` and
`stop-opacity` are read through the CSS cascade, including presentation
attributes, inline `style`, and supported class selectors. Offsets and opacity
are finite, normalized, and monotone. User-space coordinates resolve
percentages against the root viewBox and use the SVG defaults for linear
endpoints and radial center, radius, and focal point. Radial gradients with a
non-centered explicit focal point, gradient transforms, inherited paint
servers, unsupported units, external references, unresolved stop classes,
unsupported matched CSS declarations, `currentColor`, and inherited stop
values fail with a stable unsupported error. A singular referencing transform
also returns `unsupported` rather than sampling in an undefined coordinate
space.

No external resource is loaded. `svg_shape_document_destroy` releases the
paint-server stops.
