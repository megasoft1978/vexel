---
id: WO-RUST-V1-SCOPE-01
revision: 1
status: READY
milestone: V1-SCOPE
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 6489608e4a12f762d12b8032361e5afaaf73f181170a36d8c6729ea25d1b20a3
parallel_safe: false
network: false
---
# Freeze a concise, complete V1 workflow

## Goal
Replace the overly deep V1 milestone definition with a short scenario-first
vertical slice that demonstrates each product pillar in one small character
workflow. This is a first delivery milestone inside the unchanged full Rust
application goal.

## Required decision
Add accepted `ADR-0029` defining V1's scope and cutline. Revise
`docs/product/vexel-v1-vertical-slice.md` to specify one small character and one
short animation loop, basic disclosed SVG/Inkscape support, editable `.vexel`
save/reopen, animated export, and local visible MCP control of the same open
project. The user must be able to follow the MCP operation, see the drawing
update, revise it, animate it, save/reopen it, and export it.

Keep the UI task-focused and consistent with the accepted custom macOS shell:
canvas-led Draw/Animate views, a few direct tools, a compact inspector/timeline,
clear sentence-case actions, keyboard access for the core path, and visible MCP
activity/result. Use Blender only for useful workspace/timeline ideas; do not
copy its interface chrome.

Explicitly defer broad SVG/CSS/effect fidelity, advanced path/node editing,
large-file performance goals, full project recovery/migration, general asset
management, rich animation authoring, remote MCP, broad client matrices, final
UI/E2E, release packaging, and the final Codex demo video until their full-goal
gates. Preserve unsupported SVG source where possible and disclose unsupported
content. Do not claim that V1 clears full-product requirements.

Provide a feature/time table and a total estimate grounded in existing roadmap
evidence. Use focused-engineering ranges, not date promises. The app shell is
accepted, but live native window/text appearance remains unverified and must be
included in integrated V1 acceptance. Keep broad automated UI/E2E at the final
full-product stage; V1 gets focused code checks and one integrated walkthrough.

Update only the V1 summary section of `docs/product/rust-port-goal-and-plan.md`.
Do not modify its canonical full goal, full completion gates, Rust inventory,
release requirements, or live Codex/README/website video criteria. Keep the full
product goal active and unchanged.

## Allowed paths
- `work/WO-RUST-V1-SCOPE-01.md`
- `docs/adr/0029-v1-vertical-slice-cutline.md` (new)
- `docs/product/vexel-v1-vertical-slice.md`
- `docs/product/rust-port-goal-and-plan.md` (V1 summary section only)
- `artifacts/WO-RUST-V1-SCOPE-01/**` except frozen source seed and scope hashes
- `STATE.md`, `LEDGER.md`, `ledger.jsonl` (coordinator records after verification)

## Protected paths
All source code, tests, dependencies/manifests, `.github`, website/README,
ADR-0018 and other ADRs, full goal JSON, historical work orders/evidence, all
non-V1 portions of the Rust plan, all other product plans, and every unrelated
dirty change. Do not change the full objective or any final acceptance gate.

## Verification
- `python3 scripts/wo_validate.py WO-RUST-V1-SCOPE-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check` for tracked documentation edits plus direct whitespace checks
  on new ADR/order files.
- Check every included item remains inside V1's stated narrow subset and every
  deferred item remains explicitly required by the full goal.
- Check every estimate row rolls up to the stated total and clearly distinguishes
  V1 acceptance from final full-product acceptance.
- Record exact document hashes, command output, and scope audit. No application
  tests, browser automation, UI/E2E, native launch, or performance runs.

## Output
Leave changes uncommitted. Update `STATE.md` and both ledgers after the V1 scope
and roadmap validate. Keep the full Rust application goal active.
