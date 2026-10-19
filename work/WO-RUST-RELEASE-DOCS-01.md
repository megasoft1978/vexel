---
id: WO-RUST-RELEASE-DOCS-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: 9e030caf17dca33ceeb7ba7a0e560e0c3e013feb
spec_sha256: e045b6b11d075fbf82eb035983a39c5276f8a520bc6bcb79f7fc4cf6fd489012
parallel_safe: true
network: false
---
# Goal
Reconcile `docs/release.md` with the current Rust `vexel-dev` packaging and CI workflow so maintainers can follow only source-confirmed commands and understand the release limits.

# Context and locks
- Follow `OWNER-DIRECTIVES.md`: Apple Silicon macOS 13+ only; Rust workflow; no unsupported portability or signing claims.
- Read `rust/tools/vexel-dev/src/{args.rs,main.rs,bundle.rs,release.rs}`, `.github/workflows/ci.yml`, `work/WO-RUST-V1-RELEASE-CI-01.md`, and its result evidence.
- Preserve C-era instructions only in explicitly labeled migration history if needed; they must not be presented as current release steps.
- Do not invent flags, commands, signing capability, bundle identity, app icon behavior, live GUI verification or distribution support.
- No tests, builds, packaging, install smoke, network access, or GUI checks in this documentation slice.

# Allowed paths
- `docs/release.md`
- `work/WO-RUST-RELEASE-DOCS-01.md` and `ledger.jsonl` by coordinator only

# Protected paths
All source, manifests/locks, CI workflows, packaging outputs, other docs/work orders/evidence, legacy sources, tests/oracles and external paths.

# Acceptance
- AC-1: documented command names, arguments, defaults and output paths match the current Rust implementation.
- AC-2: describe the actual prerequisite order and distinguish bundle, DMG, SBOM and install smoke; state that install smoke does not launch the GUI.
- AC-3: accurately disclose ad-hoc signing and owner-gated Developer ID/notarization, and any unresolved release identity/artwork requirements.
- AC-4: remove obsolete CMake, `./dev`, Python packaging and removed workflow instructions from current usage guidance.
- AC-5: no broader product readiness, clean-install launch, GUI or full migration claim is made.
- AC-6: `git diff --check` passes and the work-order path validator accepts the exact changed-path set.

# Verification
`git diff --check`; `python3 scripts/wo_validate.py --ready WO-RUST-RELEASE-DOCS-01`.
