---
id: WO-QA-03
revision: 2
status: READY
milestone: M3
owner: project-owner
executor: deepseek-flash
base_sha: ef09a95423604fc4b41a9802bb9a4f3c94511cc7
spec_sha256: e4983d56fb6ec7fc580875d22b191a46d70d9a4d0b8498dfbcd318c442230ce8
parallel_safe: false
network: false
---
# Goal
Restore a passing, deterministic M3 owner-packet validator. `python3 reports/M3/validate_packet.py --check` currently fails with `changed source hash: docs/PERF.md`, because the M4 renderer work legitimately updated the living performance document after the M3 packet pinned its bytes.

# Context pointers
- Read: `AGENTS.md`, `OWNER-DIRECTIVES.md`, `reports/M3/validate_packet.py`, `reports/M3/evidence.json`, `reports/M3/decision.md`, `docs/PERF.md`, `work/WO-M3-05.md`.
- The generator's own comment reads: "Frozen input bytes. Updating a source requires an explicit reviewed update here."
- Investigation for this order: exactly one of the pinned inputs is stale. Every other entry in `PINNED` and `M0_EXTRA_HASHES` still matches the worktree; the sole mismatch is `docs/PERF.md` (pinned `7e9e6ec7…`, actual `a581fdcf…`).
- `docs/PERF.md` is a living narrative document that every milestone's performance work appends to; it is not a frozen measurement source. The packet's numeric claims come from the JSON sources, not from that document's prose.

# Existing contracts and invariants
- The M3 packet must remain an accurate record of what M3 asserted. Frozen measurement inputs (`bench/results/**`, `artifacts/WO-M*/**`, `work/WO-M*.md`, ADRs) stay enforced byte-for-byte; only the living narrative is reclassified.
- `--check` must remain deterministic and must still fail when a genuinely frozen input changes.
- No measurement result, claim, or hash of a frozen input may be altered. No pinned evidence file may be regenerated other than the packet's own two derived outputs.
- No new dependency; Python standard library only.

# Files allowed to touch
- `reports/M3/validate_packet.py`
- `reports/M3/evidence.json`
- `reports/M3/decision.md`
- `tests/test_m3_packet.py`
- `artifacts/WO-QA-03/**`
- `STATE.md`
- `LEDGER.md`
- `ledger.jsonl`

# Protected paths
- `work/**`, `OWNER-DIRECTIVES.md`, `AGENTS.md`, `docs/adr/**`, `src/**`, `app/**`, `CMakeLists.txt`, `tests/oracle/**`, `tests/golden/**`, `bench/**`, `docs/PERF.md` itself, every other `artifacts/WO-*/**` path, `reports/M0/**`, `reports/M4/**`, and every path outside the allowlist.

# Required implementation
1. In `reports/M3/validate_packet.py`, separate the input set into **frozen** inputs (enforced byte-for-byte, exactly as today) and **living** inputs. Move `docs/PERF.md` to the living set and document why in the generator's own comment. A living input must be listed in the packet's `sources` as `{"path": ..., "living": true}` with no byte hash, so the packet stays byte-reproducible no matter how the narrative document is later edited; a frozen input keeps its exact `{"path", "sha256"}` entry and is still enforced. Update `validate_schema` and `render_markdown` for the living entry shape.
2. Keep every other input in the frozen set unchanged, including their hashes.
3. Regenerate the packet with `python3 reports/M3/validate_packet.py --write` and inspect the diff of `reports/M3/evidence.json` and `reports/M3/decision.md`. The diff must change only the `docs/PERF.md` entry (its hash replaced by the living marker) and its row in the Markdown source table. If anything else changes, stop and record the unexpected delta instead of committing.
4. Add `tests/test_m3_packet.py`, which runs the validator's functions in-process and asserts: (a) the current tree passes the frozen check; (b) mutating a frozen input's expected hash makes the check fail; (c) a simulated change to a living input does **not** make the frozen check fail, and the living input is still listed in the packet's source table; (d) the packet contains no hash for the living input.
5. Record in `artifacts/WO-QA-03/` the before/after validator output, the diff summary, and the justification for reclassifying `docs/PERF.md`.

# Acceptance criteria (machine-checkable)
- AC-1 `python3 reports/M3/validate_packet.py --check` exits 0 and reports the same number of sources as before this order.
- AC-2 `python3 -m unittest tests.test_m3_packet` exits 0 and covers the four cases in Required implementation item 4.
- AC-3 `git diff --name-status` lists only allowlisted paths, and no frozen input besides the packet's two derived outputs changed: in particular `git diff --name-only -- bench/ artifacts/ docs/PERF.md work/` is empty.
- AC-4 `./dev verify` and `./dev t --all` exit 0.
- AC-5 `python3 scripts/ledger.py validate ledger.jsonl`, `python3 scripts/wo_validate.py WO-QA-03 --ready` and `git diff --check` exit 0.
- AC-6 A fresh read-only verifier reports pass against the exact candidate commit and confirms no frozen measurement or hash was altered.

# Verification commands
Run sequentially from the repository root unless the command changes directory itself.

```text
python3 reports/M3/validate_packet.py --check
python3 -m unittest tests.test_m3_packet
./dev verify
./dev t --all
python3 scripts/ledger.py validate ledger.jsonl
python3 scripts/wo_validate.py WO-QA-03 --ready
git diff --check
```

# Non-goals
No change to any M3 claim, measurement, threshold, or frozen input hash. No rewrite of `reports/M0/**` or `reports/M4/**`. No change to `docs/PERF.md`. No new dependency.

# Budget
Executor max runs: 2; verifier max rounds: 2; elapsed hard limit: 120 min; dependency/network: none.

# Failure and escalation
If regenerating the packet changes anything beyond the `docs/PERF.md` entry, do not commit: record the delta in `artifacts/WO-QA-03/` and stop for planner review, because that would mean the packet's derived claims depend on the living document and the reclassification needs a different design.

# Output
One implementation commit using `fix(reports): separate frozen and living M3 packet inputs`. Include the before/after validator output and diff summary under `artifacts/WO-QA-03/`, update `STATE.md` and both ledgers, and provide the schema-valid executor report.
