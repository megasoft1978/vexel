# vexel-svg-decode

Revision 2 is an incomplete continuation of WO-RUST-SVG-DECODE-01. It borrows
canonical LexicalDocument/DecodeSource data and owns an iterative scene draft and
traversal stack. It implements geometry/custom publication, inherited CSS style,
transforms and viewports, switch selection, local use expansion and source host
maps, ignored diagnostics, groups, and typed lower-operation request boundaries.
Definition plans enqueue source-only child expansions. Completed child scenes
are sealed before a single-use pattern ticket is offered. Finish requires Complete.

The production composition layer must supply actual DecodeOperations and native
MediaProvider implementations. Test delegates are explicit fixtures only. The
crate adds no dependencies and forbids unsafe code.

This candidate is **not accepted or complete**. Group-filter planning lacks a
pre-geometry owner contract. Completed pattern tiles lack the instance paint
handoff. AttachMask posts lack a group target. Accepted synchronous APIs prevent
a strict total-work quantum guarantee. One original smooth-path assertion fails
because accepted parser subpaths overlap while the scene validator forbids it.
Additional capacity accounting and original parity tests remain unfinished.

See artifacts/WO-RUST-SVG-DECODE-01/result.md, parity-map.json, coverage.json,
verification.json and r2/accepted-contract-blockers.md. Original mixed renderer,
CLI, cache and oracle inputs are preserved in r2/original-assertion-inputs.json;
actual integration execution remains mandatory. R1 source and reports are
preserved in r2/r1-preserved. Independent review and fresh coordinator checks
have not been performed.
