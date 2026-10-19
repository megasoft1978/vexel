---
id: WO-RUST-CI-CHROME-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 9e030caf17dca33ceeb7ba7a0e560e0c3e013feb
spec_sha256: 98b8bdd0bcc978274ce04784d8d7a0d04f122c28cd52aa0485d18aea7be1c6f2
parallel_safe: true
network: false
---
# Goal
Diagnose the hosted macOS chrome golden mismatch at SHA `9e030caf17dca33ceeb7ba7a0e560e0c3e013feb` without altering goldens, comparison rules, thresholds, production behavior, or CI runner selection.

# Evidence and context
- Hosted run `39297932450` on `macos-15-arm64`, macOS `15.7.9` (`24G830`), runner image `20260907.0337.1`, failed only `vexel-app/tests/ui_chrome_goldens.rs::chrome_frames_match_first_party_goldens`.
- It reports every draw/animate, dark/light, empty/compose-v1, 960x640/1280x800, 1x/2x case; differing-pixel rates remain below 0.5%, but maximum channel deltas are 38–91, above the frozen 24 ceiling.
- The current local host is Apple Silicon macOS `27.0.1`; the suite passed there previously. The test explicitly renders CoreText text and mentions OS variation.
- The pushed native wave changed neither this test, its fixtures, `render.rs`, layout/paint, nor the Apple text renderer. This does not establish that OS font rendering is the cause; verify before proposing a change.
- Read the existing golden policy in `work/WO-RUST-UI-CHROME-01.md`. Do not infer that OS variation alone permits changing the frozen threshold or replacing fixtures.

# Parallel slices and exclusive outputs
- `environment`: compare hosted and local OS/toolchain/architecture/fixture hashes and relevant run evidence. Write only `artifacts/WO-RUST-CI-CHROME-01/environment.md`.
- `test-diagnostics`: add opt-in failure-only mismatch diagnostics to `rust/crates/vexel-app/tests/ui_chrome_goldens.rs` (such as bounding boxes and region counts). Write supplemental notes only to `artifacts/WO-RUST-CI-CHROME-01/diagnostics.md`. Do not change frame generation, goldens, tolerance, assertions, test cases, or normal-run output; no output files are written unless the diagnostic environment variable is set.
- `text-rendering`: source-backed analysis of font selection, fallback, scaling, CoreText/CoreGraphics context state and premultiplied compositing. Write only `artifacts/WO-RUST-CI-CHROME-01/text-analysis.md`. Do not implement a speculative workaround.
- Coordinator alone may later authorize a separate fix order after these independent slices identify a concrete defect or establish legitimate profile variation.

# Allowed paths
- `rust/crates/vexel-app/tests/ui_chrome_goldens.rs` (failure-only diagnostics only)
- `artifacts/WO-RUST-CI-CHROME-01/**`
- `work/WO-RUST-CI-CHROME-01.md` and `ledger.jsonl` by coordinator only

# Protected paths
All production sources, current and existing UI golden fixtures, test tolerances/assertions/cases, workflows, manifests/locks, dependencies, other tests/oracles, other work orders/evidence, preserved legacy sources, STATE/LEDGER markdown, and external paths. No fixture blessing, threshold changes, test skipping, runner/OS changes, new dependencies, packaging or GUI tests.

# Acceptance
- AC-1: environment report cites exact hosted run facts and local observations and distinguishes facts from hypotheses.
- AC-2: any test diagnostic is opt-in, failure-only, bounded in output, leaves existing comparisons and normal fixture behavior unchanged, and local focused test passes.
- AC-3: source analysis ties conclusions to exact Rust symbols and identifies whether observed evidence isolates glyph rasterization or indicates a production rendering defect.
- AC-4: no golden, oracle, renderer, UI behavior or workflow is changed; no OS-only explanation is asserted without supporting mismatch-location evidence.
- AC-5: `git diff --check` and `python3 scripts/wo_validate.py --ready WO-RUST-CI-CHROME-01` pass. Verification is focused; do not run the whole workspace suite in this diagnostic milestone.

# Verification
```text
cargo test -p vexel-app --locked --test ui_chrome_goldens -- --test-threads=1
python3 scripts/wo_validate.py --ready WO-RUST-CI-CHROME-01
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```
