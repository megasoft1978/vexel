# WO-RUST-V1-DECODE-SUBSET-01 result

Candidate only; independent review pending. Not a full-parity claim.

## Subset accepted
svg root with viewBox/width/height/preserveAspectRatio; g; transform (nested);
path; rect (rx/ry); circle; ellipse; line; polyline; polygon; solid
fill/stroke (color, none, opacity, width/cap/join/miter/dash) via attributes and
inline style; unknown namespace attributes, editor metadata (sodipodi/inkscape),
metadata and defs children preserved byte-for-byte with stable offsets.

## Evidence
Decoder R11 seeded unchanged (0 files differ); 93 tests pass. Engine 38 tests
pass (6 inherited, 32 new). Definitions suite 40 pass. Frozen commands, fmt, WO
validation and ledger validation exit 0. `cargo tree` shows only vexel-* path
crates. Only production change: engine `src/lib.rs`.

## Deferred behavior
- filter, marker: engine fails closed with Unsupported at the element. The
  definitions planner returns a FinishGroup post-op that the Decoder's filter
  path rejects (InvalidArgument), so filters cannot work end to end; markers
  expand but are not subset-verified.
- mask: Unsupported (planner reports offset 0 for group owners; engine maps it
  to the mask element).
- pattern: referenced pattern suspends (NeedPattern); never success; no renderer.
- text/image: Unsupported at element. External url(): Unsupported at value.
- Unreferenced filter/mask/marker/pattern: preserved, shapes published.
- Unknown elements in the render tree: Unsupported. <style> selectors, SMIL,
  use: decode without panic; application not verified.

## Remaining gaps
Production paint/definitions composition coverage (mask on groups, filter
contract mismatch, pattern paint handoff, integrated clip history), whole-render
output/memory gates, full original-suite parity (DECODE partition stays open),
renderer. Unexecuted: clippy (not a repo gate), corpus, UI/E2E, main-tree
integration reruns, independent review.
