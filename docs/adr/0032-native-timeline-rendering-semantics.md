# ADR-0032: native timeline rendering and application history semantics

- Status: accepted
- Date: 2026-10-19
- Decision owner: project coordinator under the accepted ADR-0022 full Rust port
- Related: ADR-0018, ADR-0020, ADR-0022, ADR-0029

## Context

ADR-0020 defines native timeline-v1 data, but its rendering and authoring
semantics are not complete enough to implement playback or drawing composition
without guessing. The preserved `legacy-c-final` migration reference specifies
these behaviors in `src/project/README.md` and
`src/project/project_animation_render.c`. This ADR records the Rust parity
contract; it does not claim the Rust implementation has achieved parity.

## Decision

- Native timeline sampling uses integer ticks in `[0, duration_ticks)`. Values
  hold the first or last key outside the keyed interval. Between keys,
  `step`, `linear`, and `cubic` use step, linear, and smoothstep
  `3t^2 - 2t^3`, respectively. Hidden property tracks are skipped.
- Property tracks are applied in serialized timeline order to a temporary,
  source-preserving SVG document. Later writes have the precedence defined by
  that order. RGBA `fill` and `stroke` writes set their color attribute and
  corresponding opacity attribute; an explicit opacity track can therefore
  override an earlier color-channel opacity write. The ordinary SVG cascade
  still applies after presentation attributes are updated. Transform values
  overwrite the presentation `transform` with a six-coefficient matrix.
- SVG target IDs are resolved against source XML identity. Flattened scene
  indexes and transient document handles are not persistent identities. A
  drawing track's target validates identity only; it does not position, clip,
  inherit opacity for, or stack its drawing asset.
- Drawing assets are complete `image/svg+xml` documents with independent ID
  scopes. Each referenced asset must pass the owned SVG parser and the
  fail-closed script, external-reference, duplicate-ID, and unresolved-local-
  reference checks. Asset root pixel dimensions and every viewBox coordinate
  must equal the scene's corresponding values; no scaling or cropping occurs.
  Unreferenced imported assets remain opaque provenance.
- At a tick, the active drawing cel is the last cel whose start tick is less
  than or equal to that tick. No earlier cel means the track contributes no
  layer. Visible active drawing assets are rendered in timeline order and
  composited over the base scene using premultiplied source-over arithmetic
  with integer `+127` rounding. Cumulative base and drawing pixel bytes for a
  frame are capped at 512 MiB; overflow or cap failure yields no partial frame.
- Native low-level frame rendering does not apply looping, onion skin, export
  policy, or playback clock policy. The app playback layer floors elapsed time
  to integer ticks and clamps the end sample to the final valid tick. Onion
  skin is paused editor preview behind the active frame and never changes
  exported pixels.
- SVG SMIL remains a separate continuous-seconds/source-span evaluator. Native
  timeline sampling must not be routed through or merged with SMIL evaluation.
- The application uses one global undo/redo journal for source and native
  project edits. This intentionally supersedes the legacy Animate-first
  workspace undo routing. An operation that inserts an SVG ID and creates its
  timeline track is one atomic journal transition.

## Consequences

New Rust work orders must cite this ADR and keep sampling, SVG source mutation,
drawing composition, app playback, and SMIL as separate responsibilities.
Existing schema data is unchanged; future schema changes still follow
ADR-0020. Implementations must add focused tests for the specified semantics,
but verification is deferred until the currently planned implementation wave
is complete.

## Provenance

The behavior above is transcribed from the project's own preserved
`legacy-c-final` tag (`src/project/README.md` and
`src/project/project_animation_render.c`) and reconciled with accepted
ADR-0020 and the one-time Astra design review on 2026-10-19. No external code,
algorithm, or dependency is introduced.
