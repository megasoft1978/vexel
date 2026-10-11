---
id: WO-BOOT-01
revision: 1
status: READY
milestone: M0
owner: project-owner
executor: codex-gpt-6-luna
base_sha: b90dfbc44623e9834852b19513116149c83a15f0
parallel_safe: false
spec_sha256: 952034debd767495900aa971033534bfb0835dcc48872d70af1c98861c7a10e0
---
# Goal
Create a deterministic, dependency-free C17 scaffold that configures and builds a version-reporting CLI on the current Mac.

# Context pointers
- Read: `docs/research-r4/01-operating-model.md`, `docs/research-r4/02-work-order-template.md`, `docs/research-r4/05-revised-roadmap.md`, `docs/research-r3/plan/03-repo-layout.md`, `docs/research-r3/plan/04-coding-standard.md`.
- Relevant ADRs: none yet; locked product decisions are in the owner instruction and research packet.
- Frozen tests/oracles: none exist yet; no oracle files may be added or modified by this WO.
- External docs: none required. Do not fetch source or dependencies.

# Existing contracts and invariants
C17; no runtime dependency; no configure-time network access; strict compiler warnings are errors; generated files stay below ignored `build/`.

# Files allowed to touch
- `CMakeLists.txt`
- `CMakePresets.json`
- `cmake/**`
- `app/**`
- `AGENTS.md` (bootstrap rules; must link to `OWNER-DIRECTIVES.md`)
- `.gitignore`
- `README.md`
- `dev`
- `work/WO-BOOT-01.md`
- `STATE.md` and `LEDGER.md` (initial project status only)
- `docs/PERF.md`
- `bench/results/WO-BOOT-01.json`
- `artifacts/WO-BOOT-01/**`

# Protected paths
- `tests/oracle/**`
- `tests/golden/manifest.lock`
- `bench/corpus/manifest.lock`
- `docs/research-*/**`

# Required implementation
1. Add a minimal CMake project with required ISO C17 and extensions disabled.
2. Build a small CLI named `svgworkbench` whose `--version` output is exactly `svgworkbench 0.0.1` followed by a newline; reject other arguments with a nonzero status.
3. Treat `-Wall -Wextra -Wpedantic -Wconversion -Wsign-conversion -Wshadow -Wstrict-prototypes -Wmissing-prototypes` as errors on Clang/GCC; do not add dependencies or downloads.
4. Add Ninja presets for Mac debug, release, and size-oriented builds. Add a thin executable `./dev build` entry point for the debug preset.
5. Document prerequisites and the exact configure/build/version commands.
6. Ignore all generated build and repo-local cache content.

# Acceptance criteria (machine-checkable)
- AC-1: `./dev build` exits 0 from a clean checkout; evidence `artifacts/WO-BOOT-01/ac1-build.txt`.
- AC-2: CMake configure identifies C standard 17 with extensions off, and strict warnings are enabled as errors; evidence `artifacts/WO-BOOT-01/ac2-configure.txt`.
- AC-3: `./build/mac-debug/bin/svgworkbench --version` prints exactly `svgworkbench 0.0.1\n` and exits 0; evidence `artifacts/WO-BOOT-01/ac3-version.txt`.
- AC-4: Two independent clean builds (Debug and Release) succeed; both resulting CLIs meet AC-3; source tree has no generated tracked changes; evidence `artifacts/WO-BOOT-01/ac4-clean-builds.txt`.
- AC-5: No third-party or network-fetch target is introduced; `git diff --check` succeeds and all changed paths are allowed; evidence `artifacts/WO-BOOT-01/ac5-scope.txt`.
- AC-6: `AGENTS.md` references `OWNER-DIRECTIVES.md` and states the macOS-only platform boundary; evidence `artifacts/WO-BOOT-01/ac6-agent-rules.txt`.

# Verification sequence
1. Configure/build debug via `./dev build`.
2. Check exact CLI output and unsupported-argument failure.
3. Delete preset build directories and build Debug and Release independently; repeat version check.
4. Inspect CMake cache, compile commands, diff, and generated/tracked paths.
5. Record compiler/CMake versions, executable size, and `time` for the version command as initial baseline; no speed claim.

# Non-goals
No UI, parser, rasterizer, tests framework, benchmark runner, CI, vendored library, or source downloads.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 90 min; output-token soft cap: 10000; dependency/network: none.

# Failure and escalation
Stop if strict warning flags cannot be supported without weakening the policy. Record exact compiler output and a BLOCKERS.md entry if unresolved after three attempts.

# Output
One commit `build: scaffold C17 project`; compact executor and fresh-verifier JSON reports; append a BOOT-01 entry to `LEDGER.md` and update `STATE.md` after verification.
