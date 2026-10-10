# SVG paint servers

This module resolves bounded local linear and radial gradients used by a
shape's computed fill. The stop array belongs to the decoded shape document;
the temporary definition cache is destroyed during decode cleanup. Sampling is
deterministic in the scalar rasterizer and maps pixel coordinates back through
the referencing element's inverse transform. `objectBoundingBox` coordinates
use the element's untransformed local bounds.

Linear and radial gradients support 2–256 monotone finite stops, stop opacity,
`objectBoundingBox` and `userSpaceOnUse`, and `pad`, `reflect`, or `repeat`
spread. Invertible affine `gradientTransform` values are supported. Radial
gradients accept `fx`/`fy`; omitted focal coordinates follow the resolved
`cx`/`cy`. Gradient attributes inherit through local `href="#id"` and bound
`xlink:href="#id"` chains up to 16 definitions. A child gradient's stops take
precedence; otherwise stops come from the nearest referenced gradient with
stops.

Stop colors use the shared CSS parser (148 names, RGB/HSL functions, alpha hex,
`transparent`, and `currentColor`) and CSS cascade for presentation attributes,
inline styles, and supported selectors. `currentColor` follows inherited
`color`, defaulting to black. An omitted `stop-color` defaults to black. Stops
are limited to 65,536 total per document; referenced definitions are cached in
a fixed 65,536-slot table with at most 32,768 entries. Cache keys include the
active viewport dimensions because percentage coordinates vary by viewport.
The gradient parser has a 4,194,304-token visit budget.

User-space coordinate percentages resolve against the active root or nested
viewport. Lengths accept unitless values and `px`, `pt`, `mm`, `cm`, `in`, and
`em` (`em` is 16 user units in the current subset). Unsupported gradient types,
external URLs, malformed chains, cycles, noninvertible transforms, invalid
stops, and unsupported CSS declarations fail with stable diagnostics. No
external resource is loaded.
