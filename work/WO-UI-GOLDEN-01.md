---
id: WO-UI-GOLDEN-01
revision: 1
status: READY
milestone: M6
owner: project-owner
executor: claude-opus-5-5
base_sha: 3164f5b9e4462198a2d445462eb5513a2511134e
spec_sha256: 0e87e40e9703bc4db67acff0766c1d0960d8538409b105bd74d23f5def2a0b50
parallel_safe: false
network: false
---
# Goal
Make the frozen native shell golden check independent of the host display so that it passes byte-for-byte on Retina hosts with WindowServer access as well as on headless hosts.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `STATE.md`, `BLOCKERS.md` (WO-M4-06 closure attempt 2026-10-12), `tests/ui/verify_native_shell_goldens.py`, `tests/ui/native_ui_render.m`.
- Observed failure: on a Retina host with a display, `native_ui_render` sizes `bitmapImageRepForCachingDisplayInRect:` from the 2x display backing scale, so every render is 2x the golden size. Forcing a 1x bitmap fixes the sizes, but text and control edges still differ, because AppKit rasterizes differently when WindowServer is reachable.
- Evidence: the unchanged renderer, run with the WindowServer and CARenderServer mach services denied by `sandbox-exec`, reproduces all 14 goldens byte-for-byte. The goldens were therefore captured headless and are correct.

# Existing contracts and invariants
- Apple Silicon macOS 13+ only. No new dependencies.
- Goldens under `tests/ui/native-shell-golden/` are frozen. The comparison stays exact byte equality.
- `native_ui_render` behavior and output for headless hosts are unchanged.

# Files allowed to touch
- `tests/ui/verify_native_shell_goldens.py`
- `artifacts/WO-UI-GOLDEN-01/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`
- `BLOCKERS.md`
- `work/WO-UI-GOLDEN-01.md` only as the frozen active work order.

# Protected paths
- `tests/ui/native-shell-golden/**`, `tests/ui/native_ui_render.m`, `tests/oracle/**`, `tests/golden/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, all other `work/**`, `scripts/**`, `src/**`, `app/**`, `CMakeLists.txt`, and all paths outside the allowlist.

# Required implementation
1. Run each `native_ui_render` invocation under `/usr/bin/sandbox-exec` with an inline profile. The profile allows everything by default and denies only the `com.apple.windowserver.active` and `com.apple.CARenderServer*` mach lookups, so AppKit renders as on a headless host.
2. If `sandbox-exec` cannot apply because the process is already inside a sandbox (stderr contains `sandbox_apply`), run the renderer directly; such hosts are already headless. Any other non-zero exit fails the check.
3. Keep `--update` semantics and exact byte comparison unchanged.

# Acceptance criteria (machine-checkable)
- AC-1: Debug configure/build pass, and CTest passes every test including `native-shell-goldens` on a host with WindowServer access. Evidence: verifier check records.
- AC-2: No golden, renderer source, or script outside the allowlist changes. Evidence: verifier path guard.
- AC-3: Frozen WO validation, ledger validation, and `git diff --check` pass. Evidence: verifier check records.

# Verification commands

```text
cmake --preset mac-debug
cmake --build --preset mac-debug
ctest --test-dir build/mac-debug --output-on-failure
python3 scripts/wo_validate.py WO-UI-GOLDEN-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
```

# Non-goals
- Regenerating goldens, changing the native renderer, changing comparison tolerance, or cross-platform work.

# Budget
Executor max runs: 1; verifier: 1; elapsed hard limit: 30 min; dependency/network: none.

# Failure and escalation
If any golden differs under the headless profile, stop and record evidence in `BLOCKERS.md`. Never regenerate goldens to make the check pass.

# Output
Commit `test: render native shell goldens headless`; ledger records ready through merged with verifier evidence.
