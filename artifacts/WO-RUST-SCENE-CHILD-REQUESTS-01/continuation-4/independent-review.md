# Independent final review — revision 2, continuation 4

Verdict: **REQUEST CHANGES**. The added admission, text and mapped-completion regressions substantially address continuation-3, and the candidate hashes and supplied successful execution evidence are consistent. Two expressly requested rejection cases still lack concrete nested regression proof. These are proof blockers, not reproduced implementation defects. Do not accept or seed the decoder from this review.

## Remaining blockers

1. **Matching stale parent draft/request pair is still untested.** In `rust/crates/vexel-scene/tests/child_request_contracts.rs:402–410`, the inner request is rolled back, its draft is consumed by `rollback_draft(child)`, and nested entry receives `(&outer, inner, &replacement)`. The consumed `inner` token never matched `outer`; this proves mismatched/replayed-token rejection, not rejection of its matching stale parent draft. The entire original token test is unchanged and none of the three added tests submits a stale matching parent pair to `begin_nested_request`. Continuation-3 explicitly identified this distinction. Preserve a child draft token through successful `commit_draft(&child, ...)`, retain its matching consumed request token, submit that matching closed pair as the proposed parent of a live current descendant, assert rejection, and demonstrate the actual live parent can still open and clean up that descendant. This isolates the live-parent/ancestry requirement rather than repeating the wrong-pair case. The `parent_live` check in `src/request.rs:81–85` appears correct; it is not independently exercised by the supplied stale-pair claim.

2. **Mapped static child outside the top request's suffix is still untested across nesting.** The new mapped test at `child_request_contracts.rs:503–683` creates its only tile at lines 555–556, after opening the child request at lines 534–536. Its rejected mint at lines 579–592 supplies the paused parent draft and therefore fails `draft_index` before `static_request_child` is reached. Its successful child mint uses a valid newly created tile. There is no call with the valid current child draft and a sealed, otherwise valid tile created before that child request's mark. Thus it cannot demonstrate that a tile permitted by the outer request is denied by the new top child request. Continuation-3 expressly requested rejection of “a child outside the current request's suffix.” The preserved `static_pattern_contracts.rs:555–568` tests old-child rejection under a single request, with no `begin_nested_request`; it does not exercise the new stack transition. Create such a tile before child-request push, deny its mint using the current child draft and valid child shape/mapping, then prove a suffix tile can mint and the parent remains recoverable. The guard at `src/request.rs:221–226` appears correct; nested regression proof is absent.

Refresh the final test hash, focused/package logs and related evidence after closing these cases. Neither coverage booleans nor the package pass count establishes unexecuted assertions.

## Disposition of requested nested authorization cases

All line references below are to the actual final file `rust/crates/vexel-scene/tests/child_request_contracts.rs`, not a purported copy in the continuation directory. There is no `continuation-4/child_request_contracts.rs` artifact; `candidate-seed.json` correctly names the repository test path.

| Case | Concrete evidence and disposition |
| --- | --- |
| Shape reservation before nesting | Lines 427–429 open the parent request and reserve a parent shape admission. |
| Parent authorization while child is top | Lines 436–443 reject parent admission, consumption and cancellation. Consumption/cancellation encounter the top-request identity check in `shape_admission_index`; minting with the paused parent draft is denied by current-draft validation. The test does not separately call `admit_shape(&child, parent_request, ...)`. |
| Child admission and consumption | Lines 445–447 mint and consume using the current child token/draft. Lines 448–452 reserve another admission, roll back the child request, then reject its consumption. Positive child cancellation and stale-child cancellation are not separate assertions; both cancel and consume share `shape_admission_index` (`shape_admission.rs:168–211`). No claim of a complete cancellation matrix is made. |
| Parent reservation and cancellation after unwind | Lines 453–461 unwind the child draft, consume the original parent reservation, mint/cancel a fresh parent admission, commit request/draft, and freeze with exactly one root shape. Rejections leave the valid reservation usable; child output does not survive cleanup. |
| Text rollback and continuation | Lines 470–483 charge 7 parent commands, deny paused parent admission, charge 11 child commands and assert 18, roll back to 7, unwind, then charge parent to 12. |
| Text commit and enclosing cleanup | Lines 485–499 charge another child by 13, commit and assert 25, commit its draft, continue parent to 27, then roll back parent to 0. This directly proves committed child counter retention and enclosing rollback restoration. |
| Wrong/current static scope | Lines 579–611 reject the paused parent draft and successfully mint under the current child; the returned input carries exactly `Some(child_request)`. Outside-top-suffix rejection remains blocker 2. |
| Closed child ticket under restored parent | Lines 607–634 save the child ticket, drop its borrow, commit child request/draft, and reject `complete_mapped` with real same-job output under the restored parent. This is a closed child continuation rejection, not a successful child publication. |
| Parent static completion after recovery | Lines 636–682 derive the actual parent mapping, mint another static ticket using the same ticket registry/job and retained sealed tile, complete it successfully, assert one published root pattern, and commit parent request/draft. Failed stale completion has not stranded the registry. Output reclamation is implemented by unchanged `pattern_publication.rs:196–205`; this new test does not separately assert the stale pixel handle was released. |
| Wrong/foreign/noncurrent/out-of-order/replayed entry | Lines 363–420 reject wrong parent draft, foreign draft/request, noncurrent child, ancestor request when a child is top, out-of-order rollback and replay, interleaving valid entry/cleanup to demonstrate recovery. Matching stale parent-pair proof remains blocker 1. |

## Preserved implementation and other revision-2 requirements

Read owner directives, the active revision-2 order, continuation-2/3 independent reviews, frozen revision copies/correction, the complete final contract test, request/draft implementation and relevant builder, shape-admission, static and mapped-publication guards. Read continuation-4 coverage, verification JSON/Markdown, result, seed and audits, focused/check/package/format/validator logs. Cross-checked prior continuation metadata, focused and package results and preserved ownership proofs. Prior review conclusions are retained only where the current hashes and assertions support them.

- All four production sources are byte-identical to the continuation-3 reviewed candidate. `request_identity` selects the top request only; `request_allows_draft` checks its live draft ancestry. Nested entry validates the current child draft, exact top parent request, live parent nonce/scene/scene-count tuple, distinct descendant scene/draft and creation after the parent mark. Opaque tokens, builder-owned SceneIds and insertion serials bind ownership/generation/source lineage. The two missing test cases above are not evidence that these guards malfunction.
- Fixed `[Option<...>; 64]` draft/request arrays and explicit depths remain in `builder.rs:37–41`. Every request occupies a distinct live draft in the same ancestry, so the maximum lineage is 64. Lines 259–284 of the test successfully open root plus 63 descendant request/draft scopes, reject the next draft and unwind all scopes LIFO. No request-metadata heap allocation, dependency or unsafe code is added; test-owned Vecs do not change the implementation's bounded storage.
- Lines 25–69 populate root plus three descendant scenes, commit requests then drafts in reverse order, freeze, and resolve all four original ShapeIds. Lines 72–124 save parent/child-prefix ShapeIds and values, remove a populated child suffix and descendant, compare original prefix values, check reduced live storage/retained capacity floor/monotonic allocation history, and continue and publish the parent. These assertions remain intact.
- Lines 224–255 commit one child then later roll back the parent and reject that committed child's SceneId while preserving the pre-request scene. Source rollback uses local marks and suffix truncation, restores ignored/text counts, and retains insertion-serial identities. No broad payload snapshot or scene-wide handle epoch was introduced. Storage/history assertions are aggregate, complemented by unchanged truncation/storage accounting and freeze reconciliation.
- Lines 311–359 inject canceled commit at the outer and each of three descendant depths, assert `ErrorKind::Canceled`, then abort/unwind without WorkControl. These are the preserved four representative depth cases, not cancellation injection at all 64 possible depths. The separate boundary test unwinds all 64 levels. Rollback/abort remain unconditional after cancellation; arithmetic is checked before taking the request mark.
- Active-request draft/scene protection scans the full stack. Current-draft validation and `writable` deny paused-parent mutation; `draft_slot` retains prefix guards. Commit rejects unresolved admissions for its own token, then pops one scope; rollback restores only its mark and removes its later scenes. Serial/epoch additions and storage arithmetic remain checked. Existing error-priority assertions are byte-preserved and pass in the supplied package log.
- Legacy `begin_request` still validates the current draft then rejects nonzero request depth before polling control (`request.rs:61–70`). Frozen `decode_publication_contracts.rs:571` remains byte-preserved. Its exact focused continuation-3 run passes 1 test with 23 filtered; the continuation-4 full package also runs that test successfully. Continuation-4 has no separate frozen-rejection-focused log, so no fresh focused legacy execution is claimed.
- The preserved `static_pattern.rs:23–43` E0502/E0505 doctests exercise the live request borrow preventing builder mutation/move; ready/animation/arbitrary-parent denial proofs are unchanged. All 10 compile-fail doctests pass in the supplied package log. The tests use actual lexical/source mapping and typed capabilities; they do not establish runtime renderer behavior.

## Exact candidate, scope and preservation audit

Independently recomputed SHA-256 and line counts; all five entries match `continuation-4/candidate-seed.json` and `scope-audit.json`:

| Path relative to `rust/crates/vexel-scene/` | Lines | SHA-256 |
| --- | ---: | --- |
| `src/builder.rs` | 406 | `93e71dc2ea3c1fd66353b8d40a889d094882a5528b6a9cb9ed100b40fcdafa50` |
| `src/decode_initialization.rs` | 465 | `7bb9b36fa52b12196b3592ed68804d9d7588192b29aaddbe14bb2ecdd470ecfb` |
| `src/draft.rs` | 142 | `8455c0fdd5cbd153780ab753d5f31f4000945ef150f178fe09505b6ea56be294` |
| `src/request.rs` | 229 | `2824537c7a2db3f6f0d00d41081a824c5e305bae5cc4ea3fd3a494f8fa53e1cc` |
| `tests/child_request_contracts.rs` | 683 | `c73f7e7374ffbc38ff46d604a461bcbb1498df36c37f7c714282391ccb71ec7b` |

All are allowed paths and below 800 lines. Rehashed all **375 unique peer paths**: **371 unchanged**, exactly the four allowed production-source differences above, no missing paths or protected mismatches. All 61 accepted-main entries occur in the peer map with identical baseline hashes. Every existing Scene test, frozen assertion/public literal and static borrow proof is byte-preserved; the only new file under `rust/` is the permitted child contract test. The historical 436 figure double-counted overlapping baselines; the corrected continuation-4 audit is accurate.

All four workspace inputs match their baseline hashes:

| Path | SHA-256 |
| --- | --- |
| `Cargo.toml` | `23509968b44aed12c6ed56934c7131e539e11e51f0f6d2bddf5d8c87c3a918d5` |
| `Cargo.lock` | `6679c6fc369af4248f29146a7172f31af9c5a4d80675f5fa15f650f943803a42` |
| `rust/crates/vexel-scene/Cargo.toml` | `9a055c86e61af92236a524fbcec33a98e91ed640b1bf4411ebaf121f07ade7e7` |
| `rust/crates/vexel-scene/README.md` | `c0c88a661720d07aa73db074aff456a5371e9b297a19985c7aae825b3d7aba90` |

Root README remains `34f70610364f798bcfa9d036118eba3e4789bb241371cf84b57a775856bb84ac`. HEAD is `19234a036ed6a8d40c2ccdaec9b8186806d70eca`; tracked and staged diffs are empty. Rust/artifact inputs are untracked in this isolated worker, so peer hashes, rather than empty git diffs, establish preservation. Active order, `scope-freeze-r2.md` and continuation-2 frozen order are byte-identical at file hash `add32da0fa9aa0057145171f49408794de718993f961eab14db7b879be839383`. The validator confirms frozen spec hash `bc66b60ceddf787ed8e1e18a2714aa781ce27aadf23ddc60f7b80103096ee863`.

Prior evidence remains present and was not written by this review. Historical logs retain their original results and known corrections: continuation-1 package 225, continuation-2 package 227, continuation-3 package 229; each has 19 successful result blocks. Prior workspace-wide format failure is informational under revision 2, not a current gate. No pre-continuation-4 artifact digest inventory is supplied, so this review does not independently attest historical byte-preservation of every prior artifact or writer model/process state.

## Execution evidence and independent checks

Independently parsed all 19 continuation-4 package result blocks: **13 unit + 209 integration + 10 compile-fail doctests = 232 passed**; zero failed, ignored, measured or filtered. Per-target counts in package order are **13, 7, 11, 9, 16, 19, 24, 14, 4, 21, 3, 14, 23, 16, 4, 19, 5, 0, 10**. The full log SHA-256 is `318664056f71ece197be1dc99d2f4da391d36813dae6336f3ffd45d26137452f` (23,519 bytes).

`continuation-4/child-request-focused.log` lists all 11 actual final test names and reports 11 passed, no failures/filtering. `scene-check.log` reports successful dev-profile completion. `rustfmt.log` is empty, consistent with its recorded success; the continuation-4 recorded command covers the changed test only because the four production sources are unchanged from continuation-3. `validator.log` reports revision 2 and the spec hash above. Coverage and verification totals agree with the logs, but their assertion that continuation-3 is completely closed overstates the proof, for the two blockers above.

Supplied commands use offline locked Cargo with `CARGO_HOME=/Users/megasoft78/Desktop/Freelance/vexel/.cache/cargo` and `CARGO_TARGET_DIR=$PWD/.cache/build/rust`, as recorded in verification.md. Cargo was **not** run by this review; package/check results are supplied-log evidence. Independently reran these read-only checks, both exit 0:

```text
rustfmt --check rust/crates/vexel-scene/src/builder.rs rust/crates/vexel-scene/src/decode_initialization.rs rust/crates/vexel-scene/src/draft.rs rust/crates/vexel-scene/src/request.rs rust/crates/vexel-scene/tests/child_request_contracts.rs
python3 -B scripts/wo_validate.py WO-RUST-SCENE-CHILD-REQUESTS-01 --ready
```

Only `artifacts/WO-RUST-SCENE-CHILD-REQUESTS-01/continuation-4/independent-review.md` was written. No source/test/work-order/ledger/manifest/index/HEAD mutation, Cargo execution, UI/E2E, network, desktop interaction, nested worker or CLI model process was used. Checks were headless; no native UI behavior is verified or claimed. Fresh coordinator/main checks and exact preservation gates remain required after the proof blockers are closed.
