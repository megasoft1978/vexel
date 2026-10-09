# ADR-0009: Avoid overlapping protected and allowed work-order paths

- Status: accepted under the owner's standing authorization to repair genuine verifier findings, 2026-10-09
- Scope: WO-BOOT-08 only

## Context

WO-BOOT-08 revision 1 allowed `scripts/perf_gate.py` and `scripts/README.md` while also protecting `scripts/**`. The work-order path guard applies protected patterns before allowed patterns and does not support exceptions inside a protected glob. Independent verification therefore rejected both paths before running review, even though the implementation and machine checks were complete.

## Decision

Supersede WO-BOOT-08 revision 1 with revision 2. Keep the narrow explicit allowlist for `scripts/perf_gate.py` and `scripts/README.md`; remove the overlapping `scripts/**` protected glob. The allowlist remains fail-closed for every other script path. Do not weaken or change the path guard.

## Consequences

The revised scope language matches the runner's actual precedence semantics without expanding implementation scope. Revision 1 is terminally superseded for this preflight conflict; the same candidate changes continue under revision 2 and receive fresh verification against its frozen specification.
