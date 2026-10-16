# Independent review: WO-RUST-SCENE-CHILD-REQUESTS-01 revision 2

Verdict: **REQUEST CHANGES — frozen revision 2 is not fully met.**

The implementation is plausibly consistent with the accepted Scene ownership model, and the supplied test runs pass. Required nested-scope proof is incomplete, and the candidate manifest does not identify the final test file. These are acceptance blockers, not claims of a reproduced runtime defect.

## Concrete blockers

1. **No inner-success/outer-success regression.** `rust/crates/vexel-scene/tests/child_request_contracts.rs:131` commits the inner request, but `:145` rolls back its parent. No test in this file commits the outer request after a successful inner request, and no three-level success chain is checked. The mapped-static test (`:20`) also ends in rollback. Add a populated nested success path that commits requests/drafts in order and verifies retained data through final publication. Frozen requirement: `work/WO-RUST-SCENE-CHILD-REQUESTS-01.md:63–65`.

2. **Inner rollback does not prove prefix identity, parent continuation, or capacity/history.** The inner scope at `child_request_contracts.rs:135–143` is empty. Its assertions only check scene existence; `parent_prefix` at `:121` is an empty SceneId, not saved shape/publication/payload identity. There is no parent mutation/admission followed by successful parent completion after that rollback. There are no `memory_stats` assertions anywhere in this new test file. Existing single-request prefix/history tests in `decode_publication_contracts.rs:40–80` do not exercise `begin_nested_request`. Add populated inner rollback checks for saved parent and inner-prefix handles/data, removed suffix identities, parent reuse, retained spare capacity, released descendant payloads, and monotonic allocation history. Frozen requirement: order `:64–69`.

3. **The new entry point and top-scope authorization lack required rejection proof.** Every `begin_nested_request` invocation in `child_request_contracts.rs` is an expected success. The test named `nested_tokens_reject_foreign_stale_replayed_and_out_of_order_use` (`:256–282`) tests foreign/replayed/out-of-order *rollback* only. It does not submit wrong parent drafts, foreign/stale parent requests, replayed parent tokens, noncurrent child drafts, or out-of-order parents to the additive API. The new suite also never attempts parent shape mutation while the child request is active, nor checks parent-versus-child shape admissions/text/static ticket authority across push/pop. Add those assertions, including recovery after rejection. Frozen requirement: order `:27–31`, `:41–46`, `:66–68`. Direct code inspection supports the intended guards, but does not replace the explicitly required regression exercises.

4. **Stale final candidate test hash.** `artifacts/WO-RUST-SCENE-CHILD-REQUESTS-01/continuation-2/candidate-seed.json:27` records `9360cf9844a461c43aeb051a60cdfa47960a1203b5e4288e936e87f19f71fbc3` for `child_request_contracts.rs`. The reviewed 282-line file hashes to `3c0ed301f5512a49502c272911a81087b90ee2d045355dc37f6918a0d0987005`. All four source candidate hashes match. Refresh the final candidate record and validation evidence after closing the proof gaps. Frozen requirement: order `:102–103` (exact recorded inputs/results).

## Direct behavior and ownership review

Read the frozen order and its identical continuation-2/scope-freeze copies, accepted-main-baseline.json, workspace-peer-baseline.json, workspace-input-hashes.json, candidate-seed.json, scope-audit.json, all four changed Scene sources, the complete new test, the complete package log, both focused logs, check/rustfmt/validator logs, and continuation result. Traced the relevant accepted draft updates, admissions, pattern publication/static capability, storage, and handle APIs.

| Frozen criterion | Finding |
| --- | --- |
| Legacy `begin_request` rejects nesting | Confirmed: `request.rs:61–70` validates the current draft then rejects any nonzero request depth before polling control. Frozen rejection assertion at `decode_publication_contracts.rs:571` is hash-preserved and passes the focused run. |
| Exact parent request/draft and current descendant | `request.rs:81–101` requires `draft_index(child)`, a live matching parent scene/nonce/scene-count tuple, exact top request, distinct child draft/scene, and creation after the parent request mark. `draft.rs:59–81` and `builder.rs:171–177` bind current draft and scene owner/insertion identity. Source/generation cannot be substituted through opaque live tokens; request tokens include generation and owner-bearing SceneId. Rejection proof remains blocker 3. |
| Fixed capacity and LIFO | `builder.rs:38–41` stores two fixed 64-entry arrays and explicit depths. Request ancestry is a subset of the draft chain, so the combined lineage cannot exceed 64 levels. No request-metadata heap allocation or new dependency. `request_index` requires the current draft and exact top token. |
| Inner commit restores parent | `request.rs:147–161` validates, rejects unresolved admissions owned by that token, then removes exactly one top entry. Parent token remains unchanged. Complete success-chain proof is blocker 1. |
| Inner rollback preserves prefix | `request.rs:164–185` restores the per-scene mark and global counters, truncates only later scenes, and pops one request. `builder.rs:278–313` retains prefix serials/publications and invalidates only new admissions. Prefix/history proof is blocker 2. |
| Outer rollback removes committed descendants | Confirmed structurally by scene-count truncation; the new test `:123–147` commits a child request/draft, adds a shape, and verifies the child SceneId disappears on outer rollback. |
| Top request identity, admission and mutation ownership | `request.rs:32–47` selects only the top request and checks draft ancestry. `shape_admission.rs:77–81,173–211` uses that identity for admission/consumption/cancel. `draft_slot` (`draft.rs:91–107`) protects request prefixes; writable/draft-index checks block parent mutation while a descendant draft is current. Nested regression proof is blocker 3. |
| All active requests protect drafts/scenes | `request.rs:49–59` scans the active lineage; `draft.rs:113–132` blocks closing request-owning drafts. `builder.rs:317–357` blocks sealing protected scenes and freezing pending scopes. Descendant drafts without their own request retain the accepted ability to close before their parent request. |
| Text accounting | `request.rs:200–215` requires the current draft and a live request, bounds the global counter before checked addition, then polls before publication. Every request snapshots/restores the counter. Current drafts are necessarily at/under the top request's draft. Nested accounting assertions are absent (blocker 3). |
| Static child ticket scope | `request.rs:221–226` returns only the top request and rejects pre-request children. `static_pattern.rs:99–156` combines current draft, source/job, sealed child, viewport/mask closure, mapping and shape validation through `pattern_publication.rs`. The new real lexical/mapped-static test asserts the returned static input carries the nested token. It proves capability minting, not rendering/completion. |
| Cancellation cleanup | Confirmed: rollback/abort take no WorkControl. New test `:204–252` cancels commit at outer plus three nested depths, then aborts and unwinds without control polling. Cancellation leaves the live scope recoverable. These are empty-scope tests; populated rollback proof remains blocker 2. |
| 64-level boundary and overflow | New test `:152–177` successfully opens 64 draft/request levels, rejects the next draft, and unwinds LIFO. Bounds precede array writes. Serial and rollback-epoch additions are checked; released storage uses checked accumulation. |
| Identity, snapshots, history, priority | Request state contains a local BuildMark, scene count, ignored counters and text count; no broad payload snapshot or scene-wide handle epoch was introduced. IDs retain private insertion serials. Existing checkpoint/error ordering remains intact in the reviewed routes. Allocation/history behavior reuses accepted rollback/storage accounting; nested assertions are missing (blocker 2). |
| Compile-fail borrow and capability boundaries | Accepted `static_pattern.rs:23–43` retains E0502 mutation and E0505 freeze proofs, plus ready/animation/arbitrary-parent denial proofs. All 10 package compile-fail doctests pass in the supplied log. The same builder borrow applies when the current request is nested. No new compile-fail snippet was added; the existing directly applicable proof was preserved. |
| Frozen tests/literals/ownership assertions | Every existing Scene test hash matches accepted main; all unchanged Scene source hashes, including static borrow proofs, match. The four changed sources introduce private builder bookkeeping/additive API changes and contain no changed frozen assertions. |
| Caps, scope and README cleanup | Confirmed as detailed below; no source/test reaches 800 lines. |
| Execution and acceptance gates | Reviewed evidence uses offline Cargo and repo-local cache/target, with fixture-only claims. No GUI/UI/E2E/network/desktop test was run by this review. Fresh main checks and preservation audit remain coordinator gates after an acceptable independent review; this verdict does not authorize decoder seeding. |

## Exact verification evidence

The complete `scene-package.log` contains 19 successful result sections:

| Test target | Passed |
| --- | ---: |
| lib unit tests | 13 |
| cache_publication | 7 |
| child_request_contracts | 6 |
| consumer_contracts | 9 |
| contracts | 16 |
| decode_initialization | 19 |
| decode_publication_contracts | 24 |
| draft | 14 |
| media_source_map | 4 |
| model | 21 |
| paint_channel_contracts | 3 |
| paint_publication | 14 |
| raster_job_contracts | 23 |
| render_integration_contracts | 16 |
| scoped_media_contracts | 4 |
| shape_publication | 19 |
| static_pattern_contracts | 5 |
| support | 0 |
| compile-fail doctests | 10 |
| **Total** | **227** |

That is **13 unit + 204 integration + 10 compile-fail = 227 passed**, with **0 failed, 0 ignored, 0 measured, 0 filtered out** in the full package run. Focused child log: 6 passed, 0 failed. Focused frozen rejection: 1 passed, 0 failed, 23 filtered out. The package check log reports successful completion. The scoped rustfmt log is empty (consistent with success); the validator log reports revision 2 and frozen spec hash `bc66b60ceddf787ed8e1e18a2714aa781ce27aadf23ddc60f7b80103096ee863`.

I independently reran only these nonmutating checks, both successfully:

```text
rustfmt --check rust/crates/vexel-scene/src/builder.rs rust/crates/vexel-scene/src/decode_initialization.rs rust/crates/vexel-scene/src/draft.rs rust/crates/vexel-scene/src/request.rs rust/crates/vexel-scene/tests/child_request_contracts.rs
python3 -B scripts/wo_validate.py WO-RUST-SCENE-CHILD-REQUESTS-01 --ready
```

Cargo was not rerun because this review may write only this report; Cargo writes build/cache files. Counts above are exact supplied-log results, not a new execution claim. The stale candidate test hash is explicitly unresolved.

## Independent preservation audit

- HEAD remains `19234a036ed6a8d40c2ccdaec9b8186806d70eca`; tracked working-tree and staged diffs are empty.
- The accepted-main map has 61 entries; the peer map has 375 entries and includes all 61 (375 unique paths, not 436 unique files). Every overlapping baseline digest agrees.
- Recomputed all 375 peer digests. The only four differences are allowed Scene sources: builder.rs, decode_initialization.rs, draft.rs, request.rs. All other 371 entries match, including all 16 existing Scene integration/support files and the protected peer crates/manifests. The only new file under `rust/` relative to that map is the allowed child_request_contracts.rs.
- All four workspace-input digests match: Cargo.toml, Cargo.lock, Scene Cargo.toml and Scene README.md. Manifest/dependency/lock preservation is confirmed.
- Line counts match scope-audit.json exactly: builder 406; decode_initialization 465; draft 142; request 229; child tests 282. All are below 800.
- Root README.md is byte-identical to HEAD and hashes to `34f70610364f798bcfa9d036118eba3e4789bb241371cf84b57a775856bb84ac`. Allowed Scene README.md remains at accepted digest `c0c88a661720d07aa73db074aff456a5371e9b297a19985c7aae825b3d7aba90`. Revision-2 README cleanup claims are accurate; the stale continuation-1 root-README allowlist is historical evidence, not the current scope.
- All three frozen revision-2 order copies are byte-identical and the validator confirms their frozen spec hash. Prior evidence remains present. These supplied baselines do not independently attest historical writer process/model state or provide a pre-review digest for every prior evidence artifact.

Only this review file was written. No source, test, order, README, ledger, manifest, index or HEAD changes were made; no nested worker or extra CLI model process was launched.
