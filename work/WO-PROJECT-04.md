---
id: WO-PROJECT-04
revision: 1
status: READY
milestone: PROJECT
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: f8b36eaa7d4a155b4544ddff21fd34a1232efaaf6801fb42dc1b14a968140aca
parallel_safe: false
network: false
---
# Goal
Implement the bounded recovery-capsule codec and atomic storage primitive defined by ADR-0021 so later application work can persist a complete native project snapshot tied to its source path and saved-file baseline.

# Context
- Read `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018 through ADR-0021, current `STATE.md`, and the project bundle/file codec and atomic platform file APIs.
- Native `.vexel` save currently publishes the complete project atomically, but there is no separate recovery snapshot format or writer.
- A recovery capsule must never replace the source project. App Support path selection, periodic background scheduling, retention, restart discovery, restore UX, and final broad UI/E2E acceptance remain later application-integration work.

# Files allowed to touch
- `work/WO-PROJECT-04.md`, `docs/adr/0021-native-project-recovery-capsules.md`
- `src/project/project_recovery.h`, `src/project/project_recovery.c`, `src/project/README.md`
- `tests/project/project_recovery_test.c`, `tests/test_map.json`, `CMakeLists.txt`
- `artifacts/WO-PROJECT-04/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Application UI/runtime, platform APIs/implementations, existing project codecs, all paths outside the allowlist, frozen prior orders/artifacts, and website/deployment. Preserve all pre-existing working-tree changes.

# Required implementation
1. Define a versioned recovery capsule with a fixed, documented header containing capsule version/size, source path length, baseline source file size and modification time, capture time, project payload length, and integrity checks.
2. Encode/decode a complete `.vexel` v1 bundle (manifest, exact scene, timeline, and assets) inside the capsule. Validate all sizes, arithmetic, header flags/reserved bytes, checksums, nested project format, and an optional expected-source-path match. Reject unsupported capsule versions and malformed/truncated/trailing input without returning partial views.
3. Add atomic create/replace file publication at a caller-supplied recovery path, using the existing platform atomic-file primitives. Refuse non-regular existing targets; report whether publication committed and whether directory durability succeeded. Never mark or mutate the source project as saved.
4. Keep the capsule API pointer+length based with explicit borrowed lifetimes, finite limits derived from the `.vexel` bounds, and C sources below 800 lines. Do not add third-party dependencies.
5. Add focused tests for full project round-trip (including assets and timeline), exact path/baseline metadata, wrong-path refusal, unsupported/corrupt/truncated/overflow/trailing data, encode limits, atomic create/replace, and failed publication preserving the prior recovery capsule.
6. Document that this work order supplies storage primitives only; autosave scheduling, per-user directory creation, cleanup/retention, app integration, restore/conflict UI, and broad UI/E2E remain outstanding.

# Verification
- Run the project recovery and related project bundle/file focused CTests in Debug, ASan/UBSan, and UBSan presets.
- Build `svgworkbench` and the new focused test target with strict warnings in those presets.
- Validate this frozen order and ledger; run `git diff --check`, verify `tests/test_map.json`, and check all changed C files remain below 800 lines.
- Do not run broad UI/E2E, corpus, or performance suites.

# Output
Leave changes uncommitted. Record exact commands/results and follow-up app integration gaps in `artifacts/WO-PROJECT-04/verification.txt`, `STATE.md`, and `LEDGER.md`.
