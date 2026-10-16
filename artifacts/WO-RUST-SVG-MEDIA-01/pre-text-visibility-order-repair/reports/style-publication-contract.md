# Required custom-geometry publication contract

The production routes are `publish_image_request` and `publish_text_request`.
They require `StylePublisher`; there is no production implementation, neutral
fallback or implicit native provider. Old request functions are geometry-only
compatibility fixture helpers. Their 34 named tests remain; two rejected diagnostic expectations are corrected
to the original C behavior in assertion-transition.md.

Callbacks receive canonical ElementView (full source/tokens/end), computed
CssPaint with property source spans, actual geometry transform, independent use
host SourceMap, scene, real active DraftToken, real builder and WorkControl.
The geometry callback transfers owned transformed PathDocument/contours and
already admitted image index. Errors propagate unchanged through unconditional
request rollback. Text additionally restores TextState/cache/document commands.
No callback may publish to the prefix, another scene, or retain admissions or
owned output outside the request. Immutable/source-only adapter plans may be
retained. All variable loops require control polling/charging and bounded,
fallible actual storage. Scene capacity/work history is not rewound.

Stages and common obligations:

* owned display validation skips before callbacks/providers; display:none also
  skips malformed visibility. After begin, hidden/collapse visibility skips
  providers/geometry callbacks and closes traversal effects through finish.
  Invalid visibility is checked at the original custom-geometry boundary:
  image before fill/provider; text after layout/positions. Thus opacity errors
  precede invalid visibility, and text font/layout errors precede invalid
  visibility/fill. Source/map/ledger validation precedes all skips.
* begin: original traversal opacity Number errors, filter/mask/blend/isolation
  validation and request-local admission before font/style/layout/image work.
* prepare_geometry: fill/currentColor validation, fill-rule, shape admission,
  fill-opacity Number/rounding, stroke width/color/opacity/dash/cap/join/miter,
  paint-order, shape-rendering, vector-effect and inverse checks in original
  shape_custom order. Image before provider; text after layout and positions,
  once per outlined run. This moves hidden skip earlier for text as requested.
* publish: actual per-edge color, filled/evenodd/stroke state; preserve source
  map/host; gradient/pattern fill then stroke (pattern before gradient), actual
  inverse/bounds; image keeps IMAGE and zero solid alpha and skips gradient fill;
  geometry-local clip afterwards. Text clip is deferred to finish.
* finish: actual direct shape range, deferred direct text clip; opacity groups,
  filter/mask/blend/isolation, resolving real definition handles and preserving
  actual bounds/transforms. Enclosing decoder traversal must close groups/clip
  after descendant tspans and preserve shared parent opacity, not independently
  apply opacity to overlapping children. This per-request callback does not
  pretend to traverse descendants. Marker attachment is not invoked for media
  in the original traversal; shorthand/inherited contexts remain decoder duties.
  These common decoder effects
  are delegated, not implemented by neutral media defaults.

## Exact shared blocker: pre-geometry shape capacity admission

Original shape_custom.c calls builder_begin_shape AFTER fill-rule and BEFORE
fill-opacity/stroke/geometry/provider. Current Scene153 has append_shape only;
limits and reserve machinery are private. finalize_shape cannot replace geometry
ranges. No public fallible shape-slot lease can be acquired before geometry and
consumed by append_shape. Required minimal addition: a request/draft-scoped,
source-offset-bearing shape capacity admission lease, charged against actual
builder capacity/limits, consumed by append of real geometry and released by
request rollback. This is needed for exact shape-limit vs malformed stroke/media
and provider call priority. Shared repair belongs coordinator. No dummy/empty
shape or probe allocation is substituted. Tests of delegates' error ordering
cannot establish this missing actual builder admission capability.

Native providers, full decoder adapter and original mixed renderer/oracle suites
remain mandatory downstream proof. This candidate cannot claim acceptance or
complete exact common-pipeline parity while that shape admission gap remains.
