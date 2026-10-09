# ADR-0007: deterministic macOS pointer test injection

- Status: accepted by executor under standing owner direction, 2026-10-09
- Scope: `WO-M1-04` revision 3, test-only native pointer injection

## Context

The full native lifecycle suite intermittently failed to observe its synthetic
pointer-down event. The failure reproduced on both the accepted M1-03 Release
binary and the M1-04 candidate with the same diagnostic (`cycle 1 command 6`).
The exact comparison is retained in
`artifacts/WO-M1-04/baseline-ui-smoke-comparison.json`. Other pointer event
assertions and the production event path remain required.

## Decision

The platform's explicitly test-only pointer injection helper will dispatch the
constructed move/down/up event directly to the target content view's matching
event handler. Normal AppKit event delivery and production mouse handling are
unchanged. The caller continues to validate the emitted event type, logical
coordinates, bounds, and button.

## Consequences

The lifecycle smoke no longer depends on LaunchServices activation or
application focus to test pointer translation. The focused and full test
commands retain all existing lifecycle assertions. This does not change user
input routing in the application.
