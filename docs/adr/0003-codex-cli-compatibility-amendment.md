# ADR-0003: Codex CLI compatibility for the BOOT-04 runner

- Status: accepted by owner directive, 2026-10-08
- Scope: `WO-BOOT-04` only

## Context

The installed `codex-cli 0.160.1` does not expose `--ask-for-approval` or a
resume `--sandbox` option. BOOT-04 revision 1 required both exact flags and
therefore could not start executor or resume sessions. The owner explicitly
relaxed that requirement and supplied the supported executor and verifier
invocation patterns.

## Decision

The executor invocation uses `-m gpt-6-luna -c
model_reasoning_effort=high --dangerously-bypass-approvals-and-sandbox
--skip-git-repo-check`, scoped to the task worktree as its working directory.
The wrapper records the installed version and both help outputs before model
invocation. It captures and validates all resulting changed paths against the
WO allowlist and protected-path list, checks repository status outside
allowed paths, and rejects dirty or uncommitted candidate state. Only the
runner-managed candidate and verifier process groups may be terminated. The
verifier is a fresh `codex exec` session with `-s read-only` and the verifier
JSON schema.

This workflow control is not an OS security boundary. Post-run path guards
cannot prevent an out-of-scope write during execution, so the owner-provided
invocation is used only with a dedicated task worktree as cwd. No other Codex
CLI process is inspected or signaled.

## Consequences

The runner no longer blocks solely because `--ask-for-approval` or resume
`--sandbox` is absent. It preserves preflight evidence and fails closed on
other required capabilities, session identity, report validation, scope, and
worktree integrity. The exception does not change the preference for safer
sandboxed CLI flags when a future supported CLI exposes them.
