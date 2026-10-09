# Handoff

WO-M4-02 revision 2 is complete and fast-forward merged to `main` at `2d19d27960f524c9ff7fd888185ec7f7b3ca295f`. Independent read-only review passed without findings. Its direct verification covers Debug/Release builds, CTest 11/11, Python 192/192, native smoke/replay, ASan/UBSan, one million fuzz cases, ten librsvg fixtures, and the 58-file corpus report. Corpus remains 2/58 previewable and 56 unsupported; the 100 MiB first-preview gate fails. This is open/index evidence, not full-render parity.

WO-M4-03 revision 1 was superseded because its verification section did not match the installed runner's required `# Verification commands` format. Its incomplete attempt and candidate worktree are preserved. Focused raster tests, 17 librsvg fixtures, 1,000,000 sanitized fuzz cases, Release build, benchmark, and corpus checks passed; full tests under the executor sandbox were blocked by Metal and process-policy restrictions.

Revision 2 is now READY at `work/WO-M4-03.md`, with spec SHA-256 `7de3c3b204d3849736be520f04a344aa3260d9670bb89ead0b68d0844dc9325c`. Continue from the retained `wo/WO-M4-03` candidate worktree: copy the corrected order into its active work-order file, rerun full acceptance directly on the host, fix failures and performance regressions, commit, then run a fresh read-only review. Keep M4-01/02 evidence immutable.
