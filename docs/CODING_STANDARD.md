# C17 coding standard

This document is normative. Changes to ABI, ownership, threading, or source
preservation require an ADR.

## Names and structure

- Use C17, four-space indentation, LF, and `snake_case` identifiers.
- Prefix exported symbols with their module (`svg_`, `ui_`, `raster_`).
- Keep public contracts in headers and module ownership/tests in a short README.
- Prefer small functions with early error returns. Comments explain invariants,
  units, lifetime, and non-obvious choices.
- Keep source files below 800 lines; split modules before they grow further.

## Ownership and memory

- State whether each pointer is owned or borrowed and how long it stays valid.
- Use pointer+length slices; never infer length from a missing terminator.
- Use arenas or module allocators. Avoid allocation in hot loops.
- Check addition, multiplication, and capacity growth before allocation.
- Do not use VLAs, `alloca`, `strcpy`, `strcat`, `sprintf`, or unchecked format
  construction. No implicit null-termination assumptions.
- Do not retain pointers into growable buffers or pass unowned pointers between
  threads. OOM must not leave a partial edit committed.

## Errors, parsing, and assertions

- Malformed input, resource limits, unsupported features, and OOM are normal
  errors, not assertions.
- Assertions document impossible internal invariants only.
- Bound bytes, nodes, nesting, expansion, pixels, and work queues.
- Never load network resources or execute SVG scripts. Preserve unknown source
  spans and namespaces.
- Define behavior for overflow, NaN/Inf, invalid matrices, malformed UTF-8,
  invalid path flags, and zero divisors.
- Avoid recursion over user-controlled depth unless an explicit depth budget is
  checked.

## Concurrency and rendering

- Mutate documents on the UI thread. Workers read immutable snapshots and
  publish owned results tagged with a generation.
- Cancel, drain, and join workers before releasing snapshots or source buffers.
- Scalar rendering is deterministic and is the reference for optimized paths.
- Keep allocation out of pixel loops and record performance counters at module
  boundaries.

## Verification

Use only build, test, sanitizer, fuzz, format, lint, and benchmark commands
implemented by the active work order. Run required checks explicitly and record
their results. Benchmarks must include machine, compiler, build type, workload,
and raw measurements. Never claim a speedup without a reproducible baseline.
