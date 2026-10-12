---
id: WO-UI-GOLDEN-02
revision: 1
status: READY
milestone: UI
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 1ddaac358e510fe7d12cf05ea58a3952390512b1
parallel_safe: false
network: true
spec_sha256: 5ac4a02806f2496ebe6554d289ed96a75df9acd8f1caffcf15284601815385dc
---
# Goal
Make the integrated UI screenshot suite deterministic across the hosted macOS 15 CI runner and the current macOS 27 development host, including BetterDisplay-dependent display scaling. Preserve exact comparisons by capturing at a fixed scale and keeping explicit OS-profile baselines only where AppKit/Quartz output differs. Re-render the combined timeline UI goldens after the editor polish and Fit/Loop fix are integrated.

# Context and evidence
- Read `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `docs/CODING_STANDARD.md`, `docs/testing.md`, the source work orders `WO-UI-GOLDEN-01`, `WO-UI-POLISH-GOLDENS-01`, and `WO-UI-DOCS-02`, and the relevant screenshot verifiers.
- The integration base `1ddaac358e510fe7d12cf05ea58a3952390512b1` combines the reviewed editor polish, the reviewed Fit/Loop correction and documentation, CI nightly prerequisites, and CI golden diagnostics.
- Hosted macOS 15 diagnostic run `38149075199` compared 50 images: 30 exact, 14 native-shell mismatches with broad appearance differences, and six single-pixel/one-channel editor-family mismatches. The exact report and captures are in `artifacts/WO-UI-GOLDEN-01/hosted-macos-15/run-38149075199/`.
- The previous macOS 27 native-shell renderer output was 2x logical dimensions; the current macOS 27 host reports a 1x display and now emits 1x dimensions against 2x goldens. Both observations are retained in `artifacts/WO-UI-GOLDEN-01/local-macos-27/` and `.cache/ui-golden-audit-merged/`. BetterDisplay causality is not established; the renderer must stop depending on display scale.
- Combined-candidate focused CTest selection is 5/7: native-shell and timeline goldens fail; editor chrome, style, snap overlay, and accessibility tests pass locally. The timeline images need regeneration for the two integrated UI changes. Exact output: run record to be added under this order.

# Contracts and invariants
- Apple Silicon macOS 13+ only; C17/Objective-C 17, existing dependencies, existing app appearance and action semantics.
- Keep exact PNG comparisons and all semantic assertions. Do not increase tolerances, mask pixels, skip categories, or bless unexplained output.
- Preserve the original goldens and manifests before regenerating. Record OS/build, renderer hashes, display scale, image dimensions, hashes, and pixel metrics.
- Native shell test captures must use an explicit 1x bitmap scale independent of `NSScreen`, BetterDisplay, and attached displays. The production UI must not be changed to satisfy this test.
- Any OS-profile golden must be selected explicitly from detected/declared macOS major and have its own manifest. Preserve both macOS 15 hosted and macOS 27 local evidence; never overwrite one profile with output from the other.
- The existing CI diagnostic capture must remain after `./dev verify` and upload on failure.

# Files allowed to touch
- `work/WO-UI-GOLDEN-02.md` (freeze only)
- `tests/ui/native_ui_render.m`
- `tests/ui/verify_native_shell_goldens.py`
- `tests/ui/native-shell-golden/**`
- `tests/ui/verify_style_goldens.py`
- `tests/ui/style-golden/**`
- `tests/ui/viewer_timeline_render.c`
- `tests/ui/verify_timeline_goldens.py`
- `tests/ui/timeline-golden/**`
- `tests/editor/verify_chrome.py`
- `tests/editor/chrome-golden/**`
- `tests/editor/verify_snap_overlay_goldens.py`
- `tests/editor/snap-overlay-golden/**`
- `tests/editor/verify_accessibility_localization.py`
- `tests/editor/accessibility-localization-golden/**`
- `scripts/ui_golden_audit.py`
- `artifacts/WO-UI-GOLDEN-02/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- All production UI, document, platform, rendering, and MCP behavior outside the test-only native renderer.
- All other screenshot families and all unrelated test/oracle code.
- CMake/build presets, CI workflows, work orders other than this frozen file, README, Pages content, animation artwork, thresholds, and dependencies.

# Required implementation
1. Make `native_ui_render` allocate/render at explicit 1x output dimensions regardless of host backing scale. Verify repeated renders on the current host produce identical dimensions and bytes with the present display configuration.
2. Extend the existing manifest/verifier selection only as needed to keep exact, separate macOS 15 and macOS 27 captures for OS-dependent native/editor pixels. Reuse shared goldens where hashes prove equality. Unknown profiles must fail with a clear diagnostic, not silently compare against a different OS profile.
3. Capture the combined candidate on hosted macOS 15 using the existing diagnostic CI step. Preserve all actual/diff images and pixel metrics. Update only verified mismatches in the named families, including any `editor-chrome` mismatch exposed by the integrated polish.
4. Regenerate the four directly affected timeline screenshots and manifest from the combined source. Preserve the prior W02 and editor-polish images and report the combined image bounds/hashes.
5. Run the focused screenshot selection, `./dev verify`, `./dev asan`, and `./dev ubsan`; retain exact outputs. Resolve every failure introduced by these image families without altering thresholds or suppressing tests.
6. Obtain a fresh read-only Sol 6.1 review of the exact final candidate. Record verdict, image inspection, allowed-path audit, and ledger validation before pushing the completed candidate.

# Acceptance criteria
- AC-1: Native shell captures are fixed at 1x logical dimensions on macOS 27 with the current BetterDisplay configuration; repeat captures are byte-identical.
- AC-2: Exact selected profiles are used for macOS 15 and macOS 27; focused CTests for native shell, editor chrome/style, snap overlay, accessibility, and timeline pass on their respective hosts. No unknown profile is silently accepted.
- AC-3: Only the listed screenshot families change. Before images/manifests, actual images, hashes, dimensions, changed-pixel metrics, and visual comparison sheets are retained. No assertions, thresholds, or pass rules are weakened.
- AC-4: `./dev verify`, `./dev asan`, and `./dev ubsan` exit 0 locally; hosted macOS 15 CI and nightly sanitizer checks pass on the exact pushed candidate.
- AC-5: `python3 scripts/wo_validate.py WO-UI-GOLDEN-02 --ready`, `python3 scripts/ledger.py validate ledger.jsonl`, `git diff --check`, and the path allowlist pass.
- AC-6: Fresh independent Sol 6.1 review passes the exact candidate with no unresolved high/critical findings.

# Verification commands
```text
./dev build
ctest --test-dir build/mac-debug -R '^(native-shell-goldens|editor-chrome|editor-style-goldens|editor-snap-overlay-goldens|editor-accessibility-localization-goldens|ui-timeline-goldens)$' --output-on-failure
python3 scripts/ui_golden_audit.py --output .cache/ui-golden-audit-final
./dev verify
./dev asan
./dev ubsan
python3 scripts/wo_validate.py WO-UI-GOLDEN-02 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Non-goals
Production UI redesign, cross-platform support, adding CI runners, updating unrelated images, changing screenshot thresholds, changing app behavior, or claiming BetterDisplay caused historical mismatches.

# Budget
One implementation round, one fresh independent review, no new dependencies.

# Failure and escalation
If fixed-scale native rendering cannot preserve AppKit content while remaining independent of the active display, stop and record the smallest required test-only scope expansion. If the hosted macOS 15 capture or current macOS 27 output is unavailable, do not claim cross-host completion.

# Output
Commit the frozen READY order and lifecycle entry first. Commit the implementation as `test(ui): stabilize screenshot profiles across macOS displays`. Push and open/update the integration PR only after the exact candidate passes its local gates and Sol 6.1 review. Merge only when hosted CI and sanitizer checks pass.
