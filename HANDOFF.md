# WO-M4-06 handoff

The supplied review findings have been repaired in the dirty worktree based on
`2a484b195f8264b7b71632ef8eed6ea6cb881331`. The `<use>` composition now applies
the use transform before `x`/`y` and symbol viewBox mapping. Gradient and clip
resolution receive the current inverse affine and local bounds. Stop styles
use the CSS cascade and unsupported stop styling returns `unsupported`.
`clip-path` is applied at the declaring element; group/use clips use the whole
subtree bounds, resolve once, and intersect with nested clips. Paint and clip
reference resolution use the fixed ID index. A test allocator hook records
real total/peak allocation, fixed index, clip scratch, and bounded output
allocation during decode+render.

Boundary tests pass: use and clip source spans of exactly 64 KiB are accepted
and 64 KiB plus one returns `limit`; clip scratch of exactly 512 KiB is
accepted and one additional copy returns `limit`; 65,536 uses are accepted and
65,537 returns `limit`. The source SHA-256 remains unchanged for live-use,
paint-server, and clip documents. Group clip scratch is unchanged between one
and 16 descendant shapes. `shape.c` is 799 lines.

Evidence: Debug/ASan/UBSan shape tests pass; focused Python renderer tests pass
16/16; all 46 `rsvg-convert` fixtures pass with unchanged thresholds, zero
interior differences, max shared RGB delta 1, and max edge difference 88. The
ASan+UBSan fuzzer completed 1,000,000 cases with zero crashes, reports, or
invariant failures. Release configure/build, 5 x 21 paint benchmark trials,
and corpus run/check pass. The 58-file corpus has 2 successful previews, 56
unsupported, 0 limits, and 0 crashes.

Full Debug, ASan, UBSan, and `./dev verify` CTest runs each report 11/12; only
`index-replay` fails because this sandbox lacks offscreen Metal. The optional
full Python suite reports 189/199 due process-policy and native
LaunchServices/Metal restrictions; the focused Python suite passes. The
user prohibited nested `codex exec`, so a new independent review was not run.

No focused commits could be written because the sandbox denies changes to the
shared Git metadata. `git add` failed creating `index.lock`; temporary-index
staging succeeded, but `git commit` failed opening `COMMIT_EDITMSG`. The changes
remain uncommitted and no merge was made. See `BLOCKERS.md` and
`artifacts/WO-M4-06/commit-blocked.txt`.

The matrix and full command transcripts are under `artifacts/WO-M4-06/`.
Protected `tests/oracle/**` and `tests/golden/**` have an empty diff.
