**Decision: proceed with a decoder-local custom publication continuation. No further Scene mutation API is needed for that change. Counter observability and cross-package parity registration need separate coordinator-owned work before full decoder acceptance.**

These are source-inspection conclusions, not runtime verification. No files were edited, tests run, or workers spawned.

1. **Complete custom clip/pattern publication inside DECODE R2.**

   **Observed:** the candidate’s custom helpers parse style but omit clip handling and return only `Option<ShapeId>`. Its task machine has no custom publication continuation. The blocker correctly identifies that limitation. However, accepted Scene already supplies guarded finalizers, nested requests, explicit paint channels, `request_mapped_static`, and `complete_mapped`. See [candidate blocker](.cache/WO-RUST-SVG-DECODE-01/worker/artifacts/WO-RUST-SVG-DECODE-01/current-continuation-4/blocker-api-proof.md), [publication helpers](.cache/WO-RUST-SVG-DECODE-01/worker/rust/crates/vexel-svg-decode/src/publication.rs:249), and [static-pattern API](rust/crates/vexel-scene/src/static_pattern.rs:98).

   **Recommended smallest change:** add one decoder-owned resumable custom publication state, preferably in `src/custom_publication.rs`, with task/event plumbing in `tasks.rs`, `publication.rs`, `requests.rs`, and `session.rs`.

   The entry must receive the actual builder, borrowed active `DraftToken`, active `RequestToken`, canonical source/style/map, and admitted geometry. Its continuation stores owned data and token identities—not references into the decoder. It must:

   - Publish geometry under the existing request, then resolve fill and stroke separately through typed production plans.
   - Suspend through `StaticPatternRequest`; resume through the existing mapped ticket path.
   - Resolve and attach clip before reporting custom completion.
   - Keep the owning request open across suspension and sequential fill/stroke patterns.

   A synchronous media callback cannot transport this suspension. Add an explicit crate-local producer/continuation boundary; do not recursively invoke the decoder or conceal unfinished publication behind the existing successful return.

   **Required proof:** custom fill, stroke, both patterns, resolved/missing clip, hidden geometry, singular transforms and original diagnostic offsets; nested custom patterns; stale/wrong tokens, ticket reuse, wrong job/dimensions/generation; cancellation and late failure with output reclamation and suffix rollback. Exercise the public entry and actual task/resume path. This fits [R2’s allowed paths](work/WO-RUST-SVG-DECODE-01.md); it is an implementation decision, not evidence that the candidate is complete.

2. **Separate exact frozen predicates from currently available partial evidence.**

   | Frozen requirement | Implementable inside decoder now? | Smallest remaining dependency |
   |---|---|---|
   | Canonical source identity/unchanged bytes; exact shape counts; use span 65,536 success/65,537 failure; 1 MiB aggregate use success/+1 failure | Yes | Decoder tests and implementation only |
   | Exactly 512 shapes **and actual decoder-path lookup probes 512–1,024** | Shape count yes; complete predicate no | Add `DecodeSource::reference_lookup_probes()` forwarding its actual private index counter; expose decoder snapshot/delta |
   | 4,096 comments, 512 shapes, exactly two published stops, paint visits **4,096–37,376** | Input/counts yes; real cached-path proof unavailable in current decoder package | Current accepted paint already exposes `PaintCache.token_visits`; register a real paint adapter in a separate harness |
   | Clip child span 65,536/+1 and exact 512 KiB aggregate admission: eight succeed/nine fail | Algorithm/API exists upstream; decoder fixture proof is insufficient | Use accepted definitions planner in the cross-package harness; no new Scene API indicated |
   | Resolved-once clip allocation history and complete allocation comparisons | No | Accepted shared allocation-observation contract, then actual decoder/definitions/render adoption |
   | Tile-cache ownership, LRU, rollover, generation/scale invalidation, caps, 10,000 insertions | No, under decoder’s present dependencies | Separate tile-cache package test order |

   The original allocation assertions include nonzero clip history, exactly 256 output bytes, index bytes `131072 * sizeof(u32)`, padding-independent non-token allocation, growth bounded by `31 * 2048`, and monotonic history/peak. The one-versus-sixteen group case additionally requires equal nonzero clip allocation history. Decoder-local leases cannot establish these: Scene’s current tagged statistics report **live** tagged bytes; the C tracker accumulates tagged allocation history. Summing independent ledger peaks would also be incorrect. See [frozen allocation predicates](tests/svg/shape/test_reference.c:114) and [current accounting](rust/crates/vexel-scene/src/accounting.rs:130).

   **Dependency finding:** decoder depends on Scene/reference, but not paint, definitions, or tile-cache. Paint and definitions depend on Scene; tile-cache depends on raster/base. The blocker’s “paint is registration-only” statement describes its copied worker snapshot; [current accepted paint](rust/crates/vexel-svg-paint/src/lib.rs) has real cached resolution and token counting.

   **Least disruptive registration:** preserve original assertions and inputs in durable parity test sources. Use a separately authorized standalone harness with an isolated manifest/lock under ignored repo-local output and path dependencies on the real crates. Existing protected manifests remain untouched. Put the complete cache contract in tile-cache’s own unit tests under a separate order: its rollover setter is private and `cfg(test)`, so an external decoder test cannot use it. Preserve the original 1,024/8,192-byte caps and 10,000 iterations; existing related tests are not full frozen coverage.

3. **Execute these finite gates in dependency order.**

   1. **Coordinator evidence reconciliation.** Own only new seed/audit evidence and authorized order/ledger records. All five nested-request source/test hashes currently match **main = worker = recorded seed**. The worker’s seed JSON is absent. Preserve the historical report, append this comparison, and copy the seed with an explicit new timestamp/provenance record. Do not claim the missing prior-worker comparison occurred earlier. Separately seed accepted peer sources with complete hashes and protected-path audit.

   2. **DECODE R2 custom continuation.** Decoder writer owns only `rust/crates/vexel-svg-decode/{src/**,tests/**,README.md}` and permitted new evidence. Acceptance: the lifecycle proofs above, complete catalog mapping, unchanged original predicates, named R2 checks, independent review and preservation audit. Remain unaccepted while required parity gates are outstanding.

   3. **Small shared observability order.** Scene owns `src/source_context.rs`, focused counter tests, and a narrowly scoped allocation-observer module/accounting changes. Require actual-index counter forwarding and job-wide allocation events covering history, live overlap, cumulative tags and allocation lifetimes without double-counting borrowed storage. Prove observation neither changes execution nor substitutes estimates. Peer adoption paths must be explicitly listed in that order.

   4. **Parity harness plus tile-cache contract order.** Coordinator owns isolated harness registration; decoder owns adapters/test sources; tile-cache owns `src/lib.rs` test module and focused tests. Run exact lookup/paint bounds against accepted production peers, all clip limits/history predicates, and the complete cache contract. Required lower-renderer-dependent assertions remain named blocking obligations until real renderer composition supplies them.

   5. **Full decoder acceptance gate.** Reconcile every catalog symbol and assigned assertion with executed evidence or its mandatory mixed-composition gate; perform independent review, protected audit and fresh main checks. Then continue real renderer/composition and the full Rust product goal.

**Do not accept continuation 4, replace counters with fixture call counts, or narrow decoder/product scope.** The immediate custom-publication repair is local; the remaining evidence dependencies are concrete and separately ownable.