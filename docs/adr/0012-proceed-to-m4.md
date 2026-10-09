# ADR-0012: Proceed to renderer fidelity work

- Status: accepted
- Date: 2026-10-09
- Decision owner: project owner

## Context

The M3-05 viewer packet and M3-06 real-corpus comparison are complete. The
comparison measured 58 files and found only 2 currently previewable; 56 are
unsupported. It is open/index evidence, not full-render parity, and no customer
pilot has occurred.

## Decision

The owner directs the project to proceed through M4 and M5 after the M3-05/M3-06
evidence. Synthetic and public corpus evidence is sufficient to continue
engineering. The owner gate stays open for M4+ scope decisions; this decision
does not imply customer validation or a release decision.

Prioritize renderer work using the read-only corpus feature-frequency inventory
at `reports/M4/corpus-feature-frequency.md` and its machine-readable companion.
Implement high-coverage basic geometry and group transforms first, then
fill/stroke and style/class CSS, followed by references, gradients and clips.
Text and images remain later because their implementation and platform/font
requirements are substantially larger. Keep external reference files
read-only; record each milestone's supported-file count and fidelity evidence.

## Consequences

- M4-01 may establish common primitives and affine/group transforms as the
  foundation for maximizing corpus coverage.
- Milestone ordering may be adjusted within the M4 roadmap to reflect measured
  corpus frequency, with each frozen work order preserving its own acceptance
  criteria and explicit scope.
- No full-render parity or customer-pilot claim is authorized by this decision.
