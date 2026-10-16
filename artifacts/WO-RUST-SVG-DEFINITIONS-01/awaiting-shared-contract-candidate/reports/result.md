# WO-RUST-SVG-DEFINITIONS-01 candidate result

Status: **incomplete candidate; not accepted**.

The full frozen 79-symbol/10-suite catalog is represented in `parity-map.json` and `coverage.json`. The crate now contains typed source-range plans for clip, mask, pattern, marker, and filter definitions; source-indexed local reference resolution; full filter primitive parsing; clip queries, boundary accounting, capacity-bounded overlays, owner updates, and real draft-scoped group mutation; marker placement/CSS shorthand helpers; pattern tile/content math; and project-owned color parsing. Parsing and scene changes use safe Rust, checked arithmetic, fallible reserves, accepted `WorkControl`, actual `DecodeSource`, `ChargeLedger`, `SceneBuilder`, and `DraftToken` APIs. No shared or protected paths were changed.

Scoped verification passes: 21 unit tests and 5 real-source/scene integration tests, including exact clip 64 KiB and 512 KiB boundaries, stylesheet clip-rule inheritance, all five planner kinds, marker shorthand, tile inverse, clip-chain order/reapplication, and budget stops. All files remain below 800 lines.

One precise shared-contract limitation remains: current `vexel_scene::PatternDescriptor`, `PatternRequest`, and stored `Pattern` have no tile-origin x/y or user-to-pattern inverse fields. `PatternMapping` and the exact cache key preserve these values as owned data for the coordinator paint adapter; renderer acceptance requires that adapter to carry them into paint, or a coordinator-owned shared API extension.

The 10 original C/Python suites and their frozen inputs remain unchanged and are not claimed as run. Renderer/kernel/provider composition, original pixel/fidelity oracles, app/runtime/release and authentic MCP demonstration remain later gates. Successful verification is candidate evidence only; independent review and fresh main checks are required.
