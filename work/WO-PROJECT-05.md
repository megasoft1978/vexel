---
id: WO-PROJECT-05
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 79f5d42954271897578bdf3169e83325b7092378395754377dbc66fd5c2ac8e3
parallel_safe: false
network: false
---
# Goal
Implement the macOS recovery-store location and stable source identity path required to connect the accepted recovery capsule codec to the user's Application Support directory.

# Context
- Follow OWNER-DIRECTIVES.md, docs/CODING_STANDARD.md, ADR-0021, and WO-PROJECT-04.
- Runtime storage belongs under `~/Library/Application Support/Vexel/Recovery`, with owner-only permissions.
- Derive a collision-resistant filename from the canonical source path using the system SHA-256 implementation. Store capsules with a `.vxrecover` extension; the capsule itself retains and checks the full source path.
- Expose a testable root override so focused tests never write into the user's real Application Support folder.
- This slice does not schedule snapshots, access editor state, discover capsules at launch, or show restore UI. Background snapshot consistency first needs an immutable document-edit snapshot API; never materialize a large document on the UI thread on a timer.

# Files allowed to touch
- `work/WO-PROJECT-05.md`
- `src/platform/platform_recovery_store.h`, `src/platform/macos/platform_recovery_store.m`
- `tests/platform/platform_recovery_store_test.m`
- `CMakeLists.txt`, `tests/test_map.json`, `src/platform/macos/README.md`
- `artifacts/WO-PROJECT-05/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Existing atomic file and recovery capsule APIs, app/UI/runtime, all other platform implementations, website/deployment, frozen work orders/artifacts, and paths outside the allowlist. Preserve all existing working-tree changes.

# Required implementation
1. Add a pointer+length API that accepts an optional Application Support root override and a source path, canonicalizes the existing source path, creates the Vexel recovery directory with owner-only permissions, and returns a bounded absolute capsule path.
2. Use CommonCrypto SHA-256 over the canonical filesystem path to derive a stable collision-resistant basename; retain the `.vxrecover` extension and verify output capacity/arithmetic.
3. With no override, obtain the current user's Application Support directory using Foundation APIs; do not construct it from environment variables.
4. Reject invalid, missing, non-regular source paths; reject symlink/non-directory recovery components and unsafe permissions rather than following an attacker-controlled path. Report explicit errors.
5. Add focused tests for deterministic key, distinct paths, canonical aliases, default directory placement, override isolation, output bounds, missing source, and unsafe directory handling. Verify created directories are mode 0700.
6. Document that scheduling, background immutable snapshots, retention, restart discovery, restore/conflict UX, and broad UI/E2E remain outstanding.

# Verification
- Strictly build `svgworkbench` and the focused platform test in Debug, ASan/UBSan, and UBSan presets.
- Run only `platform-recovery-store` focused CTest and existing project recovery CTest in each preset.
- Validate this frozen order and ledger; run `git diff --check`, verify `tests/test_map.json`, and check changed C/Objective-C files remain below 800 lines.
- Do not run broad UI/E2E, corpus, or performance suites.

# Output
Leave changes uncommitted. Record exact commands/results and integration gaps in `artifacts/WO-PROJECT-05/verification.txt`, `STATE.md`, and `LEDGER.md`.
