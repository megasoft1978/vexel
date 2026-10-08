# ADR-0004: Strict structured output schema compatibility

- Status: accepted for BOOT-05 recovery, 2026-10-08
- Scope: BOOT-05 executor report schema

## Context

The first BOOT-05 executor invocation was rejected before a turn began. Codex
CLI 0.160.1 strict structured output does not permit `minProperties` in the
executor report schema. The existing runner independently checks that report
acceptance keys exactly equal the work order's acceptance IDs, and validates
that each item cites a successful command the wrapper actually ran.

## Decision

Remove `minProperties` from the schema so the same report can be used with
strict structured output. Keep the runtime exact-key and non-empty command
evidence checks unchanged, and add a regression test that rejects missing or
empty acceptance evidence. No acceptance criterion becomes optional.

## Consequences

The executor can start on the installed CLI while the local runner remains the
authoritative validator for work-order-specific acceptance evidence. This
amendment is isolated to BOOT-05 and does not change the verifier verdict
schema or general JSON Schema validation behavior.
