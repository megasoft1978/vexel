# Independent final review — revision 2, continuation 3

Verdict: **REQUEST CHANGES**. Prior findings 1, 2 and 4 are addressed. Finding 3 is partially addressed: direct nested-entry rejection and parent mutation denial are now exercised, but the expressly requested admission/text/static authorization regression coverage across nested push/pop is still absent. This is a remaining proof blocker, not a reproduced implementation defect.

## Remaining blocker

**Complete the top-scope authorization regression proof requested in continuation-2 finding 3.** The complete 421-line `rust/crates/vexel-scene/tests/child_request_contracts.rs` never calls `admit_shape`, admission consumption/cancellation, `admit_text_commands`, or `complete_mapped`. Its real mapped-static test (lines 127–220) proves successful minting under the nested token, then drops the borrow and rolls back. It does not test parent-versus-child authority or restoration/invalidation across scope changes. The parent `finalize_shape` and direct append denials at lines 87–92 do not exercise these separate paths.

Add bounded nested regressions proving:

- A parent request/admission cannot authorize admission or consumption/cancellation while a child request is top; the child token works in its own current draft; after child cleanup and draft unwind, the parent reservation/request is usable again. Check rejection leaves valid state recoverable, and stale child admissions cannot escape cleanup.
- Parent text admission is rejected while a child draft/request is current; child text charges are counted, child rollback restores the parent count, and parent admission can continue after unwind. Include committed child charges and enclosing rollback so counter lineage is directly exercised.
- Static ticket authority follows the top request through push/pop: reject the wrong parent/current draft or a child outside the current request's suffix; prove a continuation minted under a closed nested request cannot publish under the restored parent; prove valid parent operation remains possible afterward. Keep these fixture-only capability tests; no renderer acceptance is requested.

These are the unfulfilled assertion families explicitly identified in the previous REQUEST CHANGES report and covered by revision 2's top-request authorization/text/static-lineage requirements. Source inspection supports the intended guards, but the supplied successful single-request tests do not exercise the new stack transitions. Refresh final hashes and exact focused/package evidence after adding the proof.

The token test also labels line 409 stale/replayed use, but supplies the consumed inner token with the outer draft. That demonstrates rejection of a consumed, mismatched pair; it does not isolate stale parent-draft matching. Preserve a committed draft token by reference and submit its matching consumed request token as a stale parent pair if claiming independent stale-draft coverage. The existing wrong, foreign, noncurrent, out-of-order and replayed-token assertions are valid as written.

## Disposition of every prior finding

| Continuation-2 finding | Continuation-3 evidence and disposition |
| --- | --- |
| 1. Populated inner/outer success | Addressed. Lines 25–69 append real parsed-path shapes in the root and three descendant scenes, commit request then draft at each level in strict reverse order, commit the root, freeze, and resolve all four saved ShapeIds. This proves retained populated scenes/handles through publication. |
| 2. Populated rollback, prefix and history | Addressed. Lines 72–124 save parent and child-prefix ShapeIds and copied Shape values, append a child suffix and populated descendant, commit that descendant draft, roll back the child request, compare both saved values through the original IDs, reject the suffix ID and descendant scene, check decreased live storage, retained capacity floor and monotonic allocation counters, then unwind the child draft, append to the parent, commit and freeze. The memory inequalities are aggregate assertions; retained Vec capacity and payload release are additionally supported by the unchanged storage/truncation implementation and freeze's exact live-byte reconciliation. |
| 3. Entry rejection and top-scope ownership | Partially addressed. Lines 363–421 now call the additive API with wrong parent draft, foreign parent/request, noncurrent child, out-of-order parent and consumed token; valid calls and cleanup succeed after rejections. Lines 87–92 deny parent mutation while the child is active, and lines 117–123 prove parent recovery. Admission/text/static transition coverage remains the blocker above. |
| 4. Exact final test hash | Addressed. Recomputed SHA-256 is `2940b43e7ef1374147d3a772608c818e27046ad799b46d9cabb3d6ed0615dbb8`, exactly matching continuation-3/candidate-seed.json. All four candidate source hashes also match. |

## Implementation and other frozen requirements

Read OWNER-DIRECTIVES.md, revision 2 and its byte-identical frozen copies, the full continuation-2 review, the complete final contract test, all continuation-3 evidence files, and the complete source diffs for all four changed files. The accepted originals were read from the main repository only after verifying each SHA-256 against accepted-main-baseline.json and candidate-seed.json. No accepted source was modified. The decode-initialization diff is formatting plus request-array initialization/depth checks; no frozen public literal or compile-fail snippet was changed.

- `request.rs:72–102` first validates the current child draft, finds the exact live parent draft by nonce/SceneId/scene-count, requires the exact top parent request, a distinct descendant draft/scene and creation after its scene mark. Opaque tokens, builder-owned SceneIds, insertion serials, immutable source ownership and generation bind the lineage. `draft_index` and `index` reject foreign or stale current drafts/scenes. Valid entries form a subset of the live draft ancestry.
- `builder.rs:38–41` uses fixed 64-entry draft/request arrays and explicit depths. A request requires a distinct live draft, so 64 is the lineage limit, not 128 independent nesting levels. The boundary test opens the outer plus 63 descendant requests/drafts, rejects the next draft, and unwinds. No request metadata heap allocation, dependency, unsafe code or unsupported-platform work is introduced.
- `begin_request` retains its validation/error ordering and rejects nonzero request depth before polling control. The preserved frozen assertion at `decode_publication_contracts.rs:571` and its focused log confirm legacy rejection.
- Commit validates the exact top token/current draft and unresolved admissions, then pops only that request. Rollback/abort have no WorkControl and restore the local mark, ignored/text counters and descendant scene suffix. Parent rollback removes committed descendants. Cleanup checks arithmetic before taking the active mark. Prefix IDs remain insertion-serial based; scene epoch remains checkpoint validity, not whole-scene handle identity. No broad payload snapshots were added.
- `request_identity` selects only the top request; `request_allows_draft` checks live draft ancestry. `shape_admission.rs:71–74,173–211` binds mint/consume/cancel to that top identity and current draft. `draft_slot` protects the request prefix, and `writable` denies mutation of paused parents. Draft/scene protection scans all active requests; draft close, seal and freeze cannot bypass live scopes. These paths look consistent, subject to the missing regression proof above.
- Text admission requires the current draft, retains original limit/error priority and checked addition, and snapshots/restores the global count per request. Static minting selects the top request and requires its child suffix; mapped publication additionally validates current draft, source/job identity, shape and mapping. Completion compares the saved request identity with the current top and reclaims stale output. These are distinct paths from direct append and need the transition tests listed above.
- The real mapped test parses a lexical document, initializes the root, uses the actual pattern element and path source mapping, creates a sealed correctly mapped tile scene, and obtains a real `StaticPatternRequest` carrying the nested token. Existing unchanged E0502/E0505 doctests prove the request borrow blocks builder mutation/move; ready/animation/arbitrary-parent capability denials remain intact.
- Cancellation is injected at the outer and each of three descendant depths (`cancellation_abort_cleanup_succeeds_at_every_nested_scope_depth`); canceled commit returns Canceled, then abort and draft unwind succeed without control polling. These are empty-scope cancellation tests, complemented by the separate populated rollback test; no runtime GUI/rendering claim follows.

## Exact supplied execution evidence

Read the complete final focused logs, scene-check.log, scene-package.log, rustfmt.log and validator.log, along with corrected candidate-seed, peer-protected-audit, scope-audit, coverage, verification JSON/Markdown and result. Coverage booleans overstate complete closure of finding 3 as explained above.

Independently parsed all 19 package result blocks: **13 unit + 206 integration + 10 compile-fail doctests = 229 passed**, with **0 failed, ignored, measured or filtered out**.

| Target | Passed |
| --- | ---: |
| unit | 13 |
| cache_publication | 7 |
| child_request_contracts | 8 |
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

Focused child run: **8 passed**, no failures or filtering. Frozen legacy rejection: **1 passed**, 23 filtered, no failures. Check log reports successful dev-profile completion. Scoped rustfmt log is empty; validator reports revision 2 and spec hash `bc66b60ceddf787ed8e1e18a2714aa781ce27aadf23ddc60f7b80103096ee863`.

Independently reran only these read-only checks; both exited 0:

```text
rustfmt --check rust/crates/vexel-scene/src/builder.rs rust/crates/vexel-scene/src/decode_initialization.rs rust/crates/vexel-scene/src/draft.rs rust/crates/vexel-scene/src/request.rs rust/crates/vexel-scene/tests/child_request_contracts.rs
python3 -B scripts/wo_validate.py WO-RUST-SCENE-CHILD-REQUESTS-01 --ready
```

Cargo was not run in this review. The 229 count is verified supplied-log evidence, not a fresh test execution. No GUI/UI/E2E, network, desktop interaction, nested worker or additional CLI model process was used.

## Independent preservation audit

- Rehashed all **375 unique peer paths**: **371 unchanged**, exactly four allowed source mismatches (builder.rs, decode_initialization.rs, draft.rs, request.rs), no missing peers or protected mismatches. All 61 accepted-main entries are present in the peer map with agreeing baseline hashes. Existing Scene tests/assertions and static borrow doctests are byte-preserved. The only new Rust file is the allowed child_request_contracts.rs.
- All four protected workspace-input hashes match (root Cargo.toml/Cargo.lock, Scene manifest/README). Root README hashes to the recorded `34f70610364f798bcfa9d036118eba3e4789bb241371cf84b57a775856bb84ac`. No manifest/dependency or README change is part of this candidate.
- Recomputed line counts match scope-audit: builder 406; decode_initialization 465; draft 142; request 229; child contract test 421. Every changed Rust source/test is below 800 lines and within revision-2 allowed paths.
- HEAD is `19234a036ed6a8d40c2ccdaec9b8186806d70eca`; tracked and staged diffs are empty. Rust/manifests/artifact inputs are untracked in this isolated worker, so the explicit accepted-main/peer hash audit, rather than an empty tracked diff, establishes their preservation.

Only this independent-review.md was written. No source, tests, work orders, prior evidence, ledger, manifest, index or HEAD was changed. Fresh coordinator/main verification and preservation gates remain required after the proof blocker is resolved; this review does not authorize acceptance or decoder seeding.
