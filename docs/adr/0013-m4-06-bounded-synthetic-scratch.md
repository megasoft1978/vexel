# ADR-0013: M4-06 bounded synthetic scratch buffers

- Status: accepted by planner
- Date: 2026-10-09
- Scope: WO-M4-06 revision 2 only

## Context

WO-M4-06 revision 1 prohibited copying or rewriting source bytes. The
implementation needs tiny generated numeric/syntax values for selected
reference and clip operations, but must continue to borrow the input document
unchanged. The initial generated-document implementation exceeded the intended
scope because its allocation grew with document content.

## Decision

Synthetic temporary buffers are allowed only when each individual buffer is
bounded by a compile-time constant of at most 4 KiB, independent of input size.
Such buffers may contain generated syntax or numeric values only; they must not
contain copied source intervals or rewritten source bytes. Source structures
larger than this bound must be traversed by offsets/slices without copying.
Tests must assert the bound and hash the source before and after decode/render.

This amendment does not permit a generated SVG document or any allocation whose
size scales with source bytes or reference expansion size.

## Consequences

The `<use>` and clip implementations must use bounded per-operation scratch or
direct source traversal. Over-limit syntax receives a stable limit diagnostic.
The existing XML/source preservation contract remains unchanged.
