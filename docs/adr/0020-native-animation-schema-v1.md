# ADR-0020: Native animation schema v1

- Status: accepted
- Date: 2026-10-13
- Decision owner: project owner
- Related: ADR-0018, ADR-0019

## Context

The `.vexel` container already reserves a checksummed timeline JSON payload,
but that payload has no editable semantics. SVG SMIL remains source-preserving
interchange animation; it does not represent frame-by-frame vector drawings,
exposure holds, or native animation settings. The project needs a bounded model
that can later drive the timeline UI, deterministic playback/export, MCP, and
undoable authoring without changing the SVG source representation.

## V1 model

The timeline payload is a UTF-8 JSON object with `schema: "vexel.animation"`
and `version: 1`. It contains project timing settings and ordered tracks.
Unknown schema names or versions are rejected with a compatibility diagnostic;
they are never guessed, dropped, or rewritten as v1.

- Timing uses integer ticks, with a document timebase of 1..240 ticks/second.
  Duration is 1..14,400,000 ticks (at most 24 hours at 240 ticks/second).
  Integer addressing makes frame boundaries deterministic and avoids floating
  point drift. UI frame rate is a rational numerator/denominator and must map
  exactly to the timebase; otherwise the UI reports that the chosen rate cannot
  be represented rather than rounding keys.
- Track order is explicit. Track IDs are unique stable ASCII identifiers.
  Track kinds in v1 are `drawing` and `property`; names and visibility are
  presentation metadata. Tracks target a unique SVG `id` attribute in the scene
  source, never an in-memory `document_handle`. Scene IDs are validated as
  unique and dangling targets are rejected. Authoring a track for a node with
  no SVG `id` first inserts a unique ID through the normal undoable source edit.
- A drawing track contains ordered cels. A cel is an exposure beginning at a
  tick and referencing an immutable vector drawing asset. It remains exposed
  until the next cel or track end; a held exposure does not duplicate drawing
  bytes. A cel may optionally carry a bounded label.
- A property track binds a stable scene-node ID and one declared property.
  V1 scalar properties are `x`, `y`, `width`, `height`, `cx`, `cy`, `r`, `rx`,
  `ry`, `opacity`, `fill-opacity`, `stroke-opacity`, `stroke-width`, and
  `stroke-dashoffset`; `fill` and `stroke` use four-component RGBA; `transform`
  uses six affine coefficients; `visibility` uses a boolean. Keys use integer
  ticks and `step`, `linear`, or `cubic` interpolation. V1 `cubic` is the
  deterministic smoothstep curve `3t²-2t³` between adjacent keys and has no
  authored tangent payload. Numeric values must be finite; RGBA components
  must be in 0..1. Unknown properties/value-type pairs
  and interpolation names are rejected by v1 and preserved only by the opaque
  container path.
- Asset references are names in the existing project asset table, never paths.
  Drawing assets are complete SVG documents in a dedicated media type; they
  must pass the owned SVG parser and reject scripts, external references, and
  unresolved IDs. Each drawing has its own SVG ID scope. Imported source assets
  remain provenance and are not silently treated as a native drawing.
- Onion-skin settings are project display preferences (before/after counts,
  bounded to 0..8, and opacity 0..1); they do not alter rendered/exported
  frames. Playback range, loop mode, and rational frame rate are project timing
  settings.

## Limits and validation

The existing 64 MiB timeline-payload limit remains. V1 additionally caps tracks
at 256, cels/keys per track at 100,000, total cels/keys at 1,000,000, scene-node
target IDs at 256 bytes each (the borrowed scene-ID inventory is capped at one
million sorted unique IDs), nesting depth at 32, and any
string at 4 KiB. The aggregate cap takes precedence when per-track maxima sum
above it. Duplicate IDs, duplicate key ticks, out-of-duration ticks,
non-finite numbers, unsafe asset names, dangling asset/node references,
unknown enum values, malformed JSON, and arithmetic overflow are hard errors.
Validation must be allocation-bounded and report an error category and byte
offset. The current MCP JSON parser's 2 MiB/65,536-token limits do not satisfy
this 64 MiB project payload contract; project validation uses a separate
bounded streaming parser. The decoder borrows the original JSON bytes;
editing/serialization uses the project document's transactional ownership path
and preserves unrelated project/scene data. SVG `id` attributes provide
persistent target identity; transient document handles never enter the file.

## Compatibility and migration

The container remains format version 1. An empty timeline payload is treated as
an empty native animation document for projects created before this schema was
introduced. A non-empty payload not tagged with this exact schema/version is
reported as opaque/unsupported and remains byte-identical on load/save when it
is not edited. Once native animation editing is requested, unsupported payloads
must be preserved in a recoverable copy and editing refused; no lossy rewrite
is allowed. Future incompatible semantic changes increment the timeline schema
version and require explicit migrations. Container layout changes still require
a container version bump under ADR-0019.

## Alternatives considered

- Store every cel as an SVG fragment: rejected as the long-term native model
  because fragment context, IDs, styles, and references make isolation and
  deterministic composition ambiguous. SVG remains import/export.
- Store drawing bytes inline in every frame: rejected because held exposures
  duplicate large content and make edits expensive.
- Use floating-point seconds for key positions: rejected because frame-based
  authoring needs stable exact boundaries and deterministic serialized edits.
- Replace the reserved JSON section with a binary timeline immediately:
  rejected because JSON is inspectable and the existing container already
  bounds it; the first model should be simple to diagnose and evolve.

## Consequences

This decision separates SVG SMIL tracks from Vexel-native frame/exposure tracks
while allowing both to coexist in one project. SVG target-ID creation and validation, and SVG drawing-asset validation, are
prerequisites for authoring integration. Onion skins
are a preview feature, not part of exported frame pixels. This ADR is accepted
by the project owner.
