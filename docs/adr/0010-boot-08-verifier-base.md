# ADR-0010: Freeze BOOT-08 against the committed amendment base

- Status: accepted under the owner's standing authorization to repair genuine verifier findings, 2026-10-09
- Scope: WO-BOOT-08 only

## Context

The work-order verifier evaluates changed paths relative to the active work order's `base_sha`, then rejects protected paths before applying the allowlist. ADR-0009 and its work-order revision therefore must already be part of the comparison base. Revision 2 still pointed at the pre-amendment base, so verifier preflight correctly rejected the ADR as a changed protected path. The performance implementation itself remains within the narrow allowlist.

## Decision

Supersede WO-BOOT-08 revision 2 with revision 3 and set `base_sha` to the main commit that contains ADR-0009 and the path-rule amendment (642d6f5). Keep the exact same implementation allowlist, protected paths, tests, and acceptance criteria. Rebase the candidate on this base and rerun the independent verifier.

## Consequences

The verifier compares only candidate implementation/evidence changes after the amendment commit. The ADR remains immutable and protected, and the path guard is unchanged.
