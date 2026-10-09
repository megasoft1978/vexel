# ADR-0014: M4-06 bounded referenced-subtree copies

- Status: accepted by planner
- Date: 2026-10-09
- Scope: WO-M4-06 revisions 3 and 4 only
- Authority: planner continuation decision; supersedes ADR-0013 only for the
  referenced-subtree behavior required by WO-M4-06.

## Context

WO-M4-06 review identified that source-backed reference expansion must remain
bounded. Direct token traversal would avoid copying, but the current decoder
consumes a contiguous SVG slice. The planner decided to accept narrowly bounded
subtree copies for local `<use>`, clip-path, and paint-server references.

## Decision

A referenced element subtree may be copied into scratch/generated input only
when its source byte length is at most 64 KiB. Across one decode/render job,
referenced subtree copies may total at most 1 MiB. Generated wrapper syntax
remains bounded to 4 KiB. Crossing either reference budget returns a stable
`limit` diagnostic and does not crash. The source input is borrowed and never
modified. No new allocation may grow with total document size.

Tests must exercise each boundary, the over-limit diagnostic, and source SHA-256
equality before and after decode/render.

## Consequences

The `<use>` implementation now traverses the already-tokenized original source
inside shape decode. It does not copy referenced subtrees, so its per-reference
copy usage is zero. The same 64 KiB per-reference and 1 MiB per-job values cap
the source spans visited through references, bounding repeated traversal. The
clip and paint-server paths continue to read original token spans. This
direct-traversal implementation supersedes the contiguous-slice approach
described above for this work order.

The replacement limits are a fixed 131,072-slot ID index (512 KiB), at most
65,536 IDs and use instances, reference depth 16, active element visits
4,194,304, transform operations 4,096, and XML/instance depth 64. These bounds
are independent of expansion output size and are documented in
`src/svg/shape/traverse-README.md`. Generated wrapper syntax and source maps
are eliminated. Source bytes remain borrowed and diagnostics use original byte
offsets.

Revision 4 removes a contradictory blanket ADR protection from the frozen
work-order path rules; only this specifically listed amendment ADR is allowed.
