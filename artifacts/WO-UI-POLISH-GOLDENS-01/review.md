# WO-UI-POLISH-GOLDENS-01 review

- Source UI commit: `06d7fe53d39c803853e0257b178b9060d4792e35`.
- Base for this follow-up: `34ac1fe8cbb4f722b96e627610fe05c8d7f5aca8`.
- CI run before these baseline updates: 9 screenshot mismatches. Comparing with historical base run `38147398376` at `c43fa826c0fbc8910e8dc0c09b4139c8aa615995` shows four pre-existing runner failures: native shell, editor style, snap overlay, and accessibility localization. The other five were new screenshots affected by the icon and HUD paint changes.
- Ran each existing renderer in `--update` mode. No verifier code, assertions, thresholds, or update rules changed. Images and manifests with changed hashes are listed in `image-hash-diff.json`.
- Inspected old and new editor chrome images: common toolbar/tool glyphs are visibly smaller, and the status pill is shorter and narrower. The cartoon screenshot in the prior work order confirms the same app appearance at live 175% zoom.
- The native shell renderer output on this workstation doubled in pixel dimensions versus its old golden (for example, `welcome-empty-light.png` 600×650 → 1200×1300; app icon 256×256 → 512×512). BetterDisplay was enabled. The test creates an offscreen AppKit bitmap; this report records the display-scale observation without claiming the monitor caused it. The regenerated 2x output matches local renderer checks; portability of that scale remains an open host-specific risk.
- `ctest ... -R '(native-shell-goldens|editor-chrome|editor-style-goldens|ui-kit-gallery|ui-timeline-goldens|editor-command-overlay-golden|editor-snap-popover-goldens|editor-snap-overlay-goldens|editor-accessibility-localization-goldens)'` passed 10/10 on the local Apple Silicon host. The extra matched test is `ui-editor-chrome`.
- Full Debug build passed. `git diff --check`, work-order validation, and ledger validation passed. No full suite was rerun after updating baselines.
