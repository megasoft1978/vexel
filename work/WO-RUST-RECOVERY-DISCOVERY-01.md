---
id: WO-RUST-RECOVERY-DISCOVERY-01
revision: 2
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna
base_sha: b509b07ce715b5877a43033873e7cb0cd2a3dfa2
spec_sha256: 8a1733b80027fc392f5c0c770e185c5c3daa83124a75c1230f6df4e49fb76d02
parallel_safe: true
network: false
---
# Goal
Add a bounded, read-only discovery primitive for recovery capsules beside an already-open native project source.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `work/WO-PROJECT-08.md`, `rust/crates/vexel-project-io/src/recovery.rs`
- Existing capsule codec is authoritative for checksum, embedded bundle, source-path identity, and saved source stamp.
- Do not read, fetch, or copy GPL SVG-editor source. Do not fetch external SVG resources.

# Frozen public interface contract
Implement in a new private module, `rust/crates/vexel-project-io/src/recovery_discovery.rs`, these public-in-module types and function; the worker may add only the private `mod recovery_discovery;` declaration in `lib.rs` so the package tests compile; the coordinator will add public re-exports after candidate verification:

```rust
pub enum RecoveryDiscovery {
    Absent,
    Matching { capsule_bytes: Vec<u8> },
    Conflict { capsule_bytes: Vec<u8> },
    Invalid { error: RecoveryError },
}

pub fn project_recovery_discover(source_path: &[u8], capsule_path: &[u8]) -> RecoveryDiscovery
```

`Matching` means the capsule decodes with exact source-path identity and its saved source size, mtime seconds, and mtime nanoseconds match the current regular source file. `Conflict` means the capsule is valid and has the exact source-path identity, but one or more saved source-stamp fields differ. `Absent` means the expected capsule path does not exist. `Invalid` covers invalid arguments, symlink/nonregular paths, oversized or unreadable files, wrong source identity, corrupt/unsupported capsule or invalid embedded bundle. Preserve the concrete codec `RecoveryError` where available. Valid outcomes return the original bounded capsule bytes so the caller can later decode and make an explicit restore decision. This function never restores, deletes, or modifies either file.

# Invariants
- Apple Silicon macOS 13+ only; Rust only; no dependencies or platform shims.
- The caller supplies both exact paths. Do not invent a capsule naming/key policy.
- Reject empty/NUL paths, source/capsule path equality, symlinks and non-regular source/capsule files.
- Bound capsule reads by `RECOVERY_MAX_BYTES`, including a check for growth/race beyond the limit. Do not read the project source body.
- Check current source metadata at the opened file descriptor and ensure the path still identifies that regular file; compare `st_mtime` seconds and nanoseconds exactly.
- Decode with exact supplied source-path bytes. Wrong-path capsules are `Invalid { error: RecoveryError::PathMismatch }`.
- No write, cleanup, restore, UI, store lookup, missing-source identity, retention or migration behavior.
- Keep each Rust source file under 800 lines; preserve unknown bundle payloads.

# Files allowed to touch
- `rust/crates/vexel-project-io/src/recovery_discovery.rs`
- `rust/crates/vexel-project-io/src/recovery_discovery_tests.rs`
- `rust/crates/vexel-project-io/src/lib.rs` (only the exact private `mod recovery_discovery;` declaration; no public re-export)
- `artifacts/WO-RUST-RECOVERY-DISCOVERY-01/**`

# Protected paths
- All other source/tests; public module/re-exports in `lib.rs` remain coordinator-owned.
- All docs/work orders/shared ledgers/status, manifests/lockfiles, fixtures, tags, main HEAD/index, C/Objective-C references, and files outside the allowlist.
- No dependencies, network, installs, GUI actions, test weakening, resets, or deletions. Stage and commit the candidate only on the runner-owned task branch; never commit to `main`.

# Required implementation
1. Implement the frozen result enum/function contract using the existing capsule decoder and bounded filesystem metadata/read operations.
2. Add focused local-fixture tests for absent capsule, exact matching stamp, changed size/mtime conflict, wrong source path, corrupt/unsupported/invalid bundle, oversized capsule, symlink/nonregular source and capsule, and source/capsule same path. Assert discovery leaves bytes and file associations untouched.
3. Record exact commands, outcomes, and any platform-specific limits in `artifacts/WO-RUST-RECOVERY-DISCOVERY-01/result.md`.

# Acceptance criteria (machine-checkable)
- AC-1: The frozen public-in-module interface exists with the specified variants and behavior; no undocumented status branch.
- AC-2: All listed cases are covered; matching/conflict return the complete validated capsule bytes and invalid cases identify the codec error where defined.
- AC-3: Discovery is read-only and bounded; no source content reads, writes, deletes, path-policy invention, or third-party dependency.
- AC-4: `cargo +stable test -p vexel-project-io --locked --offline` exits 0.
- AC-5: `git diff --check` exits 0; only allowed paths changed; each source file is below 800 lines.

# Verification commands
```text
cargo +stable test -p vexel-project-io --locked --offline
git diff --check
```

# Non-goals
App lifecycle integration, recovery dialogs, restore/discard/save cleanup, recovery capture scheduling, store key policy, missing-source discovery, retention, broad UI/E2E, and timeline editing.

# Budget
Executor max runs: 1; verifier: 1; elapsed hard limit: 60 min; output-token soft cap: 9000; dependency/network: none.

# Failure and escalation
Stop and report exact evidence if the existing codec or macOS metadata APIs cannot support the frozen contract without dependencies or source-format changes. Do not invent fallback semantics.

# Output
Candidate only until independent read-only review. Do not update shared status/ledgers or commit to `main` from the task worktree. A task-branch commit is required for a reviewable runner candidate. Executor returns JSON report and `result.md` evidence.
