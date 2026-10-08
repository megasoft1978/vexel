# Operating model: Sonnet plans, Luna executes

## Roles and loop

- **Sonnet 5.5 planner/architect/reviewer:** writes 1–2 page WOs, identifies dependencies and contract boundaries, approves changes to architecture/specs, and reads compact verification results. Sonnet does not inspect routine raw logs or whole diffs.
- **Codex GPT-6 Luna executor/tester:** headless `codex exec`; implements exactly one WO in its task worktree, runs required build/tests/fuzz/bench commands, commits on success, and returns schema-bound report. Target >90% of model tokens here.
- **Fresh Luna verifier:** separate new `codex exec` session, read-only sandbox, no implementation context/session transcript. It reads the WO, frozen test manifest, diff, and machine artifacts; checks acceptance and spec scope adversarially. It cannot patch. The verifier is an agent, not an independent oracle: tests and reference implementations remain the objective evidence.
- **Owner:** product calls and listed hands-on gates only. Humans do not supply missing implementation labor.

```text
Owner locks product decisions
          |
Sonnet drafts/approves WO ----> frozen spec + test IDs + allowed paths
          |                                     |
          +------------------------------> Luna executor (fresh session)
                                                | code + tests + report + commit
                                      deterministic verifier commands
                                                |
                                  fresh Luna verifier (read-only, adversarial)
                                                |
                            compact JSON + diff stat + artifact links
                                                |
                              Sonnet accepts / returns / escalates
                                                |
                          owner only at declared product/release gates
```

Each WO must leave the repository buildable, tests green, a commit or explicit blocked state, and a handoff entry. No agent may mark its own test as sufficient when the WO names an external oracle.

## Work order lifecycle

1. `draft`: Sonnet names goal, precise acceptance commands, paths, out-of-scope cases, dependency/version pins, run/time/token budgets and JSON report schema.
2. `ready`: automated lint checks required fields, baseline working tree is clean, spec hash is recorded, tests/reference fixtures are immutable to the executor.
3. `running`: wrapper creates branch/worktree `wo/WO-ID`, records CLI version/model/config, starts `codex exec` with bounded timeout, `workspace-write`, no approval prompts, network disabled by default. A requested resume is same WO, same branch, known session ID only.
4. `reported`: runner captures final JSON, status/exit code, stdout/stderr summaries, changed paths, commits, test results, tokens/usage if CLI provides them, elapsed time, and artifact hashes.
5. `verified`: verifier worktree is a detached read-only copy of proposed commit; objective scripts run first; verifier inspects diff and WO and returns pass/fail/uncertain with evidence IDs.
6. `review`: Sonnet receives at most 2,000 tokens of summary plus `diff --stat`, verdict JSON, and failing evidence snippets capped at 20 lines each. It can request a targeted excerpt or escalate.
7. `merge`: clean pass only; merge commit/squash recorded to main, ledger updated, `STATE.md` and journal concise. No automatic release/deploy/publication.

## State and context budget

Versioned state files:

- `AGENTS.md`: durable top-level instructions, build commands, safety rules, generated areas, authoritative docs. Keep under 200 lines; link out for details.
- `STATE.md`: current milestone, active WO, last green SHA, blockers, next three IDs, current baseline measurements; overwrite summary, not diary.
- `work-orders/WO-*.md`: immutable after `ready`; corrections create `-r2` spec and supersede linkage.
- `ledger.jsonl`: append-only state transitions, SHA, run IDs, model/CLI versions, budgets, status, test-result artifact locations.
- `journal/YYYY-MM.md`: short decisions and surprises; no transcript dumping.
- `docs/adr/NNNN-*.md`: accepted architecture/product decisions with alternatives, evidence, date, superseding ADR.
- `artifacts/`: generated test JSON, golden diffs, benchmark CSV/metadata, crash reports; retain summaries long-term, large raw data by content hash/retention policy.

At session start executor reads only `AGENTS.md`, `STATE.md`, assigned WO, named context pointers, and relevant module README/header. Do not resume merely to preserve context; spec and git state are the source of truth. Resume when a run timed out or hit a transient tool issue and the same objective remains. Before resume, wrapper records git status/diff and emits a short progress summary; if transcript is missing or worktree changed unexpectedly, start a new session and inspect state.

## Git, integration, parallelism

- One branch and worktree per WO; main remains green. One commit per coherent WO (small internal commits allowed, report all). Never share a mutable worktree between agents.
- Parallelism: default one executor plus one verifier sequentially. Allow at most two executor WOs only when file sets/contracts are disjoint and both were explicitly declared parallel-safe; integrate one at a time. Avoid parallel work on renderer semantics, document model, core UI, golden baselines, build configuration, or shared headers.
- Worktree create from pinned main SHA; validate allowlist after run using changed paths and git status. New files also checked. Merge only if base SHA is unchanged or rebase/reverify.
- Protect tests: `tests/oracle/`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`, schemas, and verifier scripts require a separate test-change WO. Executor WO may add unit tests outside frozen oracle only when requested. Any golden update requires a baseline-change record, side-by-side image, explicit reason, and verifier pass against independent oracle.

## Budgets and escalation

Ordinary WO: target 60–120 minutes elapsed, hard 3-hour runtime, at most 2 executor attempts total plus 1 verifier; 15k Luna output tokens as soft cap where usage is exposed. Bootstrap/graphics WO: up to 4 hours and 4 executor attempts with explicit Sonnet approval. First failure may get one narrow repair attempt from verifier findings; if that attempt fails, Sonnet rewrites acceptance or splits the WO. Never auto-loop indefinitely. Worktree must be reset/rebased before retry if repair scope changes. Project-wide allowance: owner sets weekly usage cap; wrapper checks quota telemetry if available and pauses when near limit. Sonnet review: <=2k tokens ordinary; 8k milestone max.

Escalate immediately to Sonnet for: acceptance ambiguity; public API/ADR change; failed source-preservation or safety invariant; test and oracle disagreement; unplanned dependency/license; touching protected paths; unexplained benchmark regression >10%; platform API whose SDK behavior is unknown; scope expansion; two failed retries. Blockers are JSON (`blocked=true`, category, exact command/error, evidence path, needed decision, attempted alternatives), ledgered and left without fake passing commit.

## Drift controls

Every WO has a spec hash and explicit non-goals. Freeze oracle IDs and corpus hashes at `ready`; reviewer checks tests were not weakened/deleted. Module contracts live in public headers plus short README: invariants, ownership/threading, errors, limits, tests. Dependency/API research links point to official docs pinned by version. ADR required for changed boundary. Nightly audit checks stale STATE, orphan worktrees, unmerged commits, version drift, and tests whose assertions weakened. Once a milestone, fresh Luna audit receives only architecture index + git log + tests and identifies divergence; Sonnet decides if corrective work is needed.

## Framework comparison

The model adopts Anthropic’s [initializer/stepwise handoff](https://www.anthropic.com/engineering/effective-harnesses-for-long-running-agents) and [generator-evaluator](https://www.anthropic.com/engineering/harness-design-long-running-apps) lessons, but uses a durable spec/ledger plus a frozen machine oracle. Codex [`AGENTS.md`, `codex exec`, JSON output schemas, image input, and worktrees](https://developers.openai.com/codex/cli/reference) are native primitives. SWE-agent/OpenHands-style tool loops motivate constrained actions and observable environments; Aider’s small diff/test loop and spec-driven development motivate acceptance-first changes. Framework names do not prove quality; local task replay and defect rate determine whether the harness stays. Avoid turning the process into a large multi-agent framework: start with scripts and add complexity only after measured failures.
