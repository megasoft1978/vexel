# Candidate result

Complete uncommitted implementation of WO-RUST-SVG-CSS-CONTROL-01; root review and integration remain required.

Added `CssBuildSession`, `CssComputeSession`, controlled create/compute, and exact retained-capacity accounting. Legacy APIs drive the same private resumable engines with a no-stop policy. Build retains token, comment/quote, at-rule, brace, declaration trimming/importance, selector identifier and declaration-copy cursors. Compute retains rule, backward ancestor/token, attribute, class/byte comparison, offer, inline and winner/inheritance state. Each accepted unit performs bounded work; zero quantum only checkpoints. Terminal results are sticky, incomplete finish fails, and partial paint is inaccessible.

All 39 properties, original grammar, source spans, source-identity parent checks, inheritance resets, precedence, cumulative counters, error priorities and exact caps remain intact. Parent paint is copied; temporary context lifetime is independent of source lifetime. All owned buffers use safe fallible allocation and checked capacity arithmetic.

All five frozen checks passed with installed Rust 1.99 via +stable and inherited local offline caches. **63 tests passed:** 13 unchanged originals, their 13 verbatim bodies via controlled one-shot, their 13 via one-unit sessions, and 24 focused proofs. Independent baseline comparison covers 399 grammar combinations, 600 deterministic byte cases and malformed spans/CDATAs. Focused proofs cover long real progress, every phase and mid-loop stops, exact boundaries, counters, capacity slack, drop/reuse and parent/source lifetimes.

OOM evidence uses actual safe reserve capacity failure; physical allocator exhaustion was not induced. No scope adaptations. Source/tests remain below 800 lines. Exact baseline copies, parity map, source hashes and named-check outputs accompany this candidate.
