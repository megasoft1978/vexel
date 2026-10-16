# WO-RUST-SVG-MEDIA-01 R2 — blocked

Corrected original text-style failure behavior: malformed size, weight, family, spacing, font style and anchor values report Unsupported at the original source offset and in original evaluation order. Position failures retain Number. Text requests now honor their explicit end bound. Compound glyph contours preserve separate subpaths, closure and winding through actual scene commit.

All prior 11 tests remain intact. Added meaningful inherited font/tspan, quoted fallback, units, anchor/spacing, exact 256 KiB cache and cumulative/per-layout command-cap proofs for both cache representations. All 24 catalog symbols and 12 original cases remain mapped; original mixed inputs/oracles are preserved.

**Atomic cleanup is blocked by the accepted public shared API.** `mark`/`rollback` reject active drafts; same-scene nested drafts are rejected; outer rollback consumes the token and destroys its prefix. MEDIA cannot restore a failed request suffix while preserving outer authorization. Seven real lexical/animation/path/builder/draft/ledger reproducers demonstrate residual pixels, shapes, state or cache after capacity, limit and cancellation failures. They do not prove atomicity. Root must provide a scoped savepoint/rollback contract; see [atomicity-blocker.md](atomicity-blocker.md).

The full-scope audit also reproduced a missing cumulative document text-command budget: cached use 17 succeeds above the original 4 Mi command total. Accepted request/state exposes no persistent command counter; the separate cache counter cannot substitute. Root must supply that per-job budget with rollback semantics.

Locked offline check and test passed; 30 tests comprise 22 functional proofs and eight unresolved-defect reproducers. Frozen R2 WO validation passed. All 26 seeded protected paths match their authoritative hashes; every owned source/test is below 800 lines. Exact evidence is in [verification.json](verification.json), with [parity-map.json](parity-map.json) and [coverage.json](coverage.json).

R2 is not complete or accepted. Native providers, mixed renderer/CLI/C suites, CoreText/librsvg and application/runtime/release/UI validation remain later mandatory gates.
