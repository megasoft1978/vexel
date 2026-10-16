---
id: WO-RUST-SVG-ANIMATION-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: be7604977c3a77e877eb7409159e277dc71617f172bb8301fbc6651abe58f16d
parallel_safe: true
network: false
---
# Goal and full scope
Port all of src/svg/animation/animation.h, animation_internal.h, smil.c,
smil_tracks.c, evaluate.c and motion_path.c into safe std-only Rust. Preserve
all parse/evaluate/public helper semantics and original supported SMIL grammar,
limits (128 tracks, 256 values, 128 diagnostics, 3600 seconds, and every internal
motion/path/string bound), timing priority, diagnostic order/offsets, namespace
and byte-span behavior, target/xlink resolution, syncbase graph/cycles,
repeat/duration/end/fill, set/animate/animateTransform/animateMotion,
linear/spline/discrete/paced modes, additive/accumulation flags, path structure
compatibility and numeric/color/transform/motion interpolation. No substitute
simplified timeline. Unsupported tracks are preserved/reported as the C scope
defines; raw source is never rewritten. Owned arrays plus a borrowed source or
Arc source owner with spans; no self-reference, unsafe, whole-source copies or
query-time parsing. Significant allocations fallible and capacity/resource
charges checked; document safe Rust adaptations explicitly without fake OOM.
Rejected parse/eval leaves no partial result. Define canonical new exported
Rust types so the renderer and authoring ports can consume this complete scope.
Port all meaningful parse/eval assertions from tests/anim/test_animation.py and
animation_eval_cli.c, exact sample values/timing/attributes/active states; read
all relevant project-authored fixtures, including both motion fixtures. Add
explicit bounds, unsupported/sync-cycle, source lifetime/spans and deterministic
end/repeat/motion cases for public APIs. Render CLI assertions require the later
full renderer/CLI port and must be listed pending, not silently omitted or
claimed by parse/eval. Authoring.c/create and native timeline are separate
accounted scopes, not part of this read-only SMIL parse/eval port. No broad UI,
render campaign, fuzz or performance test. Split cohesive modules below800lines.

# Files allowed to touch
- `work/WO-RUST-SVG-ANIMATION-01.md` (coordinator freeze only)
- `rust/crates/vexel-svg-animation/src/**`
- `rust/crates/vexel-svg-animation/tests/**`
- `artifacts/WO-RUST-SVG-ANIMATION-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Protected paths and execution
All legacy sources/tests, peer Rust crates and APIs, manifests/config/locks,
other orders/artifacts and coordinator records. Worker writes only own crate
src/tests and artifacts. Current-source isolated tree `.worktrees/rust8-svg-animation`;
Standard existing subscription LunaHigh; no recursive agents, network, installs,
external dependencies, C shim, new unsafe, unsupported OS, stage/commit/reset/delete.
Read OWNER-DIRECTIVES/AGENTS first. Exact installed Rust1.99 via +stable,
macOS13+ AppleSilicon, local Cargo/cache/tmp. Preserve all original assertions.

# Verification
cargo +stable check -p vexel-svg-animation --locked --offline
cargo +stable test -p vexel-svg-animation --locked --offline
python3 scripts/wo_validate.py WO-RUST-SVG-ANIMATION-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check

# Output
Full uncommitted candidate; parity-map.json maps each C symbol/limit/error/source
contract and original assertion to implementation/tests or an explicitly named
later renderer/authoring scope. result.md<=400words and verification.txt exact
commands/results. Independent review and fresh main audit/checks before acceptance.
No app/renderer/animation-authoring/performance claim. Stop on allowance/rate
error; no extra credits/API billing or retry loops.
