# ADR-0008: Keep the M2-05 fuzz driver outside the protected scripts tree

- Status: accepted by owner standing work-order authorization, 2026-10-09
- Scope: WO-M2-05 only

## Context

WO-M2-05 revision 1 allowed `scripts/shape_fuzz.py` while also protecting
`scripts/**`. The work-order path guard applies protected patterns before
allowed patterns and does not support exceptions inside a protected glob. The
fresh verifier therefore rejected the candidate even though every machine
acceptance check passed.

## Decision

Amend WO-M2-05 as revision 2. Place the fuzz campaign controller at
`tests/fuzz/shape/shape_fuzz.py`, which is already inside the allowed shape
fuzz subtree. Keep `scripts/**` protected; do not weaken the verifier guard or
add an exception mechanism for this work order.

## Consequences

The revised file allowlist matches the runner's fail-closed scope semantics.
The fuzz command and campaign behavior remain unchanged. Revision 1 is
superseded due to this scope conflict; revision 2 receives a fresh verifier
review against its amended frozen specification.
