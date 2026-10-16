# WO-RUST-SVG-DECODE-01 candidate result

**Status: incomplete; not ready for work-order acceptance.** The crate now has an
owned `Decoder` transaction over `LexicalDocument`/`DecodeSource`, bounded token
steps, draft rollback on cancellation, CSS inheritance, transform application,
path parsing, limited primitive geometry, viewport and switch helpers, and
ignored-feature accounting. Seven focused Rust tests cover the compact and
separated path number assertions, malformed path offset, helper boundaries,
scene completion, and cancellation rollback.

The frozen order assigns 113 source definitions/helpers across 12 C files, six
engine operations, and 50 original assertion functions across 19 suites. This
candidate maps every catalog source symbol and original case in
`parity-map.json` and `coverage.json`, including explicit gaps. Most cases remain
unrun. Full use expansion, custom elements, complete style/paint resolution,
clip/mask/pattern/filter planning and one-shot pattern tickets, text/image
composition, native providers, source/limit parity, and mixed renderer oracles
are still required. The public decoder currently fails closed on several of
those paths; it must not be treated as full decode support.

No accepted shared-contract blocker was found that authorizes replacing the
missing behavior. The later coordinator-provided adapters remain an integration
gate, not evidence that this crate's frozen scope is complete. No manifests,
locks, shared APIs, protected tests, or scope catalog were changed; no dependency
was added.
