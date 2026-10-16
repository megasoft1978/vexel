# R2 shared contract blocker: request-local rollback inside an active draft

Reported before attempting a semantics substitution. Frozen R2 was read at
revision 2, spec SHA256 `f960b64a19d208552c1423ebdf51e4dfc4aa458588c8e2ceb675c97971d1d5ed`.
The coordinator supplied R2 after the preceding candidate writer was terminal;
this writer has not edited the work order, shared source, manifests, or seeds.

The accepted public `SceneBuilder` API cannot checkpoint and restore an append
suffix inside the caller's active draft:

- `builder.rs:197` `mark` rejects every nonzero `draft_depth`.
- `builder.rs:217` `rollback` likewise rejects active drafts, even with a mark
  acquired before the draft began.
- `draft.rs:19` `begin_draft` rejects same-scene nesting: an existing scene's
  index is less than the active parent's `scene_count`.
- `draft.rs:98` `rollback_draft` consumes the entire outer token, drops the
  prefix, and ends its authorization. MEDIA only borrows that token.
- Internal checkpoint/rollback and draft validation helpers are `pub(crate)`;
  SceneBuilder storage and DraftToken fields are private outside vexel-scene.

Root needs to supply and accept a public scoped checkpoint/rollback operation
authorized by the active DraftToken for that scene. It must restore request
suffixes (images, shapes, topology, publications and accounting), preserve
prefix handles and the active outer token, and clean up unconditionally after
cancellation or allocation/admission failure. A closure transaction with these
guarantees would also suffice. MEDIA must additionally restore its text state
and cache admissions on request failure when that operation becomes available.

`tests/atomicity_gap.rs` records actual public API rejection and late image/text
failures against real lexical, animation, path, builder, draft and ledger
fixtures. These are defect reproducers, not successful atomicity proofs. No
clone/rebuild, child-scene substitution, unchecked caller cleanup, or alternate
rendering semantics are used. R2 remains blocked despite independent parity
corrections and passing scoped checks.

## Additional full-scope accounting gap

`src/svg/shape/text.c` checks and increments `builder->text_commands` against
`SVG_SHAPE_MEDIA_MAX_DOCUMENT_PATH_COMMANDS` (4 Mi commands) on every run,
including cached layouts. Accepted `vexel_scene::TextState` has cursor,
text_bytes, glyph_count and whitespace fields but no command counter;
TextDecodeRequest exposes no other document-owned text-command budget.
`TextCache.commands` is the separate 262,144-command cache admission count and
cannot substitute for document uses. The MEDIA per-layout 4 Mi check cannot
enforce the cumulative original bound.

`tests/document_budget_gap.rs` demonstrates 17 uses of a valid 262,144-command
cached fixture layout succeeding, whereas the original rejects use 17. Root
must expose a persistent document text-command budget (or equivalent accepted
per-job state) through the request contract. Its increments must participate in
the same request rollback as text_bytes/glyph_count. No cache counter is
repurposed and no new shared state has been edited by this worker.
