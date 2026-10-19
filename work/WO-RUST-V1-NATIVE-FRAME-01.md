---
id: WO-RUST-V1-NATIVE-FRAME-01
revision: 1
status: READY
milestone: FULL-RUST-PROJECT
owner: project-coordinator
executor: codex-gpt-6-luna-high
base_sha: 849371835fa4cb8c3e48ff154778a3ec5d7de196
spec_sha256: 5055216fb9ce2e1d14dc2152e9da4d42d637a37ba0793c94989b30bf3e41be1e
parallel_safe: true
network: false
---
# Goal
Render one native timeline-v1 integer tick by applying ordered property samples to a temporary SVG source, rendering the base scene, then compositing active drawing cels through the accepted owned renderer.

# Context and contract
Read `OWNER-DIRECTIVES.md`, ADR-0020, ADR-0032, `animation/native_properties.rs`, `animation/native_drawing.rs`, `compose::compose_svg`, and `vexel-timeline::TimelineView` accessors. The caller supplies an already decoded/validated `TimelineView`, exact scene geometry, output size, and validated context-owned asset byte views. Keep schema validation and file/session ownership outside this renderer. Preserve serialized track order, CSS cascade and source-preserving temporary property application. Render property-edited source first, then visible active drawing tracks at the requested integer tick. Drawing targets remain identity-only. Onion skin, playback clock, and loop policy remain above this function. Return no partial frame after any error.

# API shape
Add a focused public function in `native_frame.rs` that accepts the evaluated source SVG bytes, `&TimelineView`, `&[DrawingAsset]`, the integer tick, output dimensions, and `SvgGeometry`; it returns `Result<Frame, NativeFrameError>`. Map typed property/drawing/compose errors into explicit enum variants without panics or stringly behavior. Construct temporary drawing-cel views in serialized order, preserving hidden-track behavior and each track's authored cel order. Do not retain pointers beyond the call.

# Allowed paths
- New `rust/crates/vexel-app/src/animation/native_frame.rs`
- New `rust/crates/vexel-app/src/animation/native_frame_tests.rs`
- Work-order artifact directory only

# Protected paths
All existing source, registrations, session/payload/history/MCP/main integration, manifests/locks, ADRs, assets, fixtures/oracles, work orders, ledger, C/Python evidence and external paths. Coordinator will register the module after delivery. Do not run builds/tests/benchmarks; author focused tests and leave them unrun.

# Acceptance
- AC-1: applies native property samples before base rendering and leaves persistent source bytes untouched.
- AC-2: converts drawing cels in serialized track order and delegates active-cel, geometry, SVG safety, output cap, atomic failure and premultiplied source-over to `native_drawing`.
- AC-3: rejects out-of-range ticks and maps property/drawing/render/allocation failures to typed `NativeFrameError`.
- AC-4: does not add SMIL, onion, loop or playback policy; no dependencies; files stay below 800 lines.
- AC-5: focused tests authored and remain unrun until all implementation slices are complete.

# Deferred verification
`cargo test -p vexel-app --locked animation::native_frame`
