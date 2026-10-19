---
id: WO-RUST-PROJECT-PAYLOAD-01
revision: 3
status: READY
milestone: FULL-RUST-PRODUCT-DATA
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 0a96a7110c9a9acf326dbf54dfc4d5de9a6e64b0
spec_sha256: 5ac848144b8c384a88305dd362fcb0bbf705a3ea70c3f25e74572f933ee0a9d5
parallel_safe: true
network: false
---
# Goal
Preserve the complete opaque `.vexel` project payload when opening, editing, saving, and reopening a project.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `docs/product/rust-port-goal-and-plan.md`, `docs/adr/0019-vexel-project-container-v1.md`
- Session implementation: `rust/crates/vexel-app/src/project_session/mod.rs`
- Bundle format and borrowed view: `rust/crates/vexel-project/src/bundle.rs`
- Existing atomic I/O: `rust/crates/vexel-project-io/src/{file.rs,file_load.rs}`
- No external SVG resources or GPL editor source.

# Existing contracts and invariants
- V1 bundles carry manifest JSON, SVG scene bytes, opaque timeline JSON, and named assets with media types/data.
- Preserve unknown manifest/timeline JSON exactly; this slice does not interpret animation payloads.
- Save edits only the scene and dimensions while retaining the other project payload fields.
- Keep `write_project(path, source, width, height)` behavior for source-only MCP callers: default manifest/timeline and no assets.
- Failed open leaves the current session unchanged. Failed save does not change the project association or mark the document saved.
- `.svg` imports remain untitled and Save never overwrites the original SVG.

# Files allowed to touch
- `rust/crates/vexel-app/src/project_session/mod.rs`
- `rust/crates/vexel-app/src/project_session/tests.rs`
- `rust/crates/vexel-app/src/project_session/payload.rs`
- `artifacts/WO-RUST-PROJECT-PAYLOAD-01/**`

# Protected paths
- All other source, tests, fixtures, docs, manifests/lockfiles, work orders, shared ledgers/status, and all C/Objective-C migration references. The coordinator owns the main branch and its index.
- No dependencies, network, installs, GUI actions, commits to `main`, resets, deletions, timeline interpretation, or UI changes. A candidate commit on the isolated task branch is required for runner verification.

# Required implementation
1. Add an owned, bounded session payload model for manifest JSON, timeline JSON, and asset names/media types/data. Keep each source field byte-for-byte.
2. Populate it when opening a `.vexel`; initialize new/SVG-imported sessions with the existing generated manifest, `{}` timeline and no assets.
3. Save the current document source/dimensions together with the retained payload using the existing atomic project I/O API. Keep `write_project` compatible for source-only callers.
4. Add regression tests for opaque unknown JSON fields and populated assets surviving open, scene edit, save, save-as and reopen; retain failed-save, failed-open and SVG non-overwrite coverage.
5. Record exact verification commands and outcomes in `artifacts/WO-RUST-PROJECT-PAYLOAD-01/result.md`.

# Acceptance criteria (machine-checkable)
- AC-1: A project with non-default manifest JSON, timeline JSON and multiple assets opens and round-trips each retained field byte-for-byte after a scene edit.
- AC-2: Save and save-as use the updated scene and dimensions while preserving manifest extensions, timeline bytes and every asset name/media type/data.
- AC-3: Source-only `write_project` output remains valid with its existing default metadata and no assets; `.svg` save does not overwrite the source.
- AC-4: Failed save/open behavior preserves session contents, path association and saved/dirty state.
- AC-5: `cargo +stable test -p vexel-app --locked --offline` exits 0.
- AC-6: `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check` and `git diff --check` exit 0; changes remain within the allowlist and add no dependency.

# Verification commands
```text
cargo +stable test -p vexel-app --locked --offline
cargo +stable fmt --manifest-path Cargo.toml --package vexel-app -- --check
git diff --check
```

# Non-goals
Timeline semantics or editing, animation capture/playback/export, recovery lifecycle, UI changes, public API changes, new dependencies, and changing the V1 bundle format.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 10000; dependency/network: none.

# Failure and escalation
Stop if the existing bundle limits cannot bound the owned payload or if byte-preserving round-trip conflicts with a validated format invariant. Report exact evidence; do not normalize or discard unknown metadata.

# Output
Candidate only until independent read-only review. Do not update shared status/ledgers or commit to `main` from the task worktree. Commit candidate changes on the isolated task branch after checks pass. Executor returns the required JSON report and `result.md` evidence.
