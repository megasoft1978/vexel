---
id: WO-RUST-NATIVE-WINDOW-01
revision: 2
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: b27d4914453a617b5c4232563b1bf64bf0cd95b0f70fa4ee85bc7b0abb2e18f1
parallel_safe: true
network: false
---
# Full native window implementation scope
Implement the ENTIRE accepted readiness plan embedded below, with exact source/
API/function map in artifacts/WO-RUST-NATIVE-WINDOW-PLAN-01/parity-map.json.
Real AppKit app/window/lifecycle, appearance/cursor/material availability,
geometry, separate key/pointer/scroll/magnify/committed/marked text and complete
NSTextInputClient, bounded queued/pending/sequenced reset/control/close, ticks,
weak routing/generation invalidation and teardown. No fake window/responder/
provider, empty-success/backend stub, parser/source rewrite or smaller scope.
Source incoming callback values Rust-owned, finite and bounded. AppKit types
remain private; no model borrow or RefCell borrow across reentrant native calls.
Use actual accepted MainThread/guard_native_call and pinned existing bindings.
Define own Rust-owned event/geometry values matching accepted UI conventions;
no vexel-ui manifest dependency required. Document internal surface attachment
seam for later presenter, no native pointer in public interfaces. Missing real
selected-feature/API contract: report specific issue, don't silently substitute.

# Files allowed to touch
- rust/crates/vexel-platform-apple/src/native_window/**
- rust/crates/vexel-platform-apple/src/lib.rs (only module declaration/reexports for native_window)
- rust/crates/vexel-platform-apple/tests/native_window*.rs
- artifacts/WO-RUST-NATIVE-WINDOW-01/**
- ignored repo-local .cache/tmp, .cache/cargo and build outputs for named checks
Coordinator owns work-order freeze/registration, STATE/LEDGER/ledger/fullplan.
Worker edits production modules/tests/ownreports only in isolated current-source tree.

# Protected and execution
All manifests/locks/config, other modules/peers, existing tests/main_thread/native
boundary files, source603/157oracles, previousorders/evidence, state/docs/ledgers/
index. No newpackage/feature/network/download/install/GPL source/Cshim/unsupported
OS/stage/commit/reset/delete/recursive agents/broadUI/E2E/GUI launch. Platform
unsafe only small audited objc2/CF boundary operations with documented ABI,
ownership/teardown and thread proof, never domain/new unchecked main marker.
Stop quota/rate errors, Standard existing subscription only, no API billing.
Source/tests<800. Preserve original7 boundary proofs. Candidate exit not accepted;
independent review/root scope checks/main named checks mandatory before merge.

# Verification
cargo +stable check -p vexel-platform-apple --locked --offline
cargo +stable test -p vexel-platform-apple --locked --offline
python3 scripts/wo_validate.py WO-RUST-NATIVE-WINDOW-01 --ready
Focused pure config/geometry/modifiers/queue/IME UTF16 ranges/source snapshots/
weak generation/teardown/callback fault and compile-affinity proofs; no GUI launch.
Record exact tests/build/source map, unavailable runtime proof honestly; full real
GUI/mac13/native presentation/menu/AX/files/text/app/MCP acceptance is later.

# Output
Complete real native window lane candidate, parity-map.json/result.md/verification;
explicit downstream dependencies and no false app/GUI/mac13runtime completion.

# Accepted concrete specification
# Ready order candidate: Rust macOS native window lane

**Status:** candidate for independent coordinator review; no implementation is authorized by this artifact. **Target:** Apple Silicon macOS 13+. **Dependencies:** accepted `vexel-platform-apple` MainThread and `guard_native_call`; audited pins already in its manifest; existing `vexel-ui` geometry/input/value types. New root kernel contract work is not a peer dependency. No new dependency, manifest change, C/ObjC shim, unsupported OS backend, GPL code, or generic second roadmap.

## Objective and public contract

Port the complete native window/lifecycle/geometry/input/IME/tick/queue/teardown lane enumerated in `parity-map.json` from `src/platform/platform_window.h` and the five `src/platform/macos/platform_window*.m` modules. Preserve observable contracts and limits while replacing public C pointers/fixed buffers with Rust-owned values. Keep all native owners private to `vexel-platform-apple`; no Objective-C type or pointer crosses the public API.

Proposed signatures (names may be module-qualified, semantics are fixed):

```rust
pub struct WindowId { pub slot: u32, pub generation: u32 }
pub enum WindowError { ExpiredOwner, ExpiredWindow, InvalidConfig,
    InvalidTextInputState, QueueFault, Native(NativeFault) }
pub struct QueueFault { pub reason: QueueFaultReason,
    pub coalesced_or_dropped: u64 }
pub enum QueueFaultReason { InputOverflow, TextInputStateMissing,
    InvalidTextRange, AllocationFailure }
pub struct EventBatch { pub events: Vec<WindowEvent>, pub pending: bool }
pub struct TextInputState { pub source_window: WindowId, pub text: String,
    pub selection_utf8: Range<usize>, pub marked_utf8: Option<Range<usize>>,
    pub marked_attributes: Vec<TextAttributeSpan>, pub candidate_rect_points: RectF }
pub struct Composition { pub text: String, pub selection_utf8: Range<usize>,
    pub replacement_utf8: Option<Range<usize>> }
struct NativeApp { state: Rc<RefCell<AppState>>, main_thread: MainThread }
struct AppState { phase: AppPhase, slots: Vec<WindowSlot>, free_slots: Vec<u32> }
struct WindowSlot { generation: u32, retired: bool, record: Option<WindowRecord> }
struct WindowRecord { native: Retained<NSWindow>, content: Retained<NSView>,
    render_view: Retained<NSView>, route: Rc<RouteState>, queue: EventQueue,
    timer: Option<Retained<NSTimer>>, text_input: Option<TextInputState> }
pub struct Window { owner: Weak<RefCell<AppState>>, id: WindowId }
pub struct WindowConfig { pub title: String, pub width_points: u32,
    pub height_points: u32, pub resizable: bool, pub initially_hidden: bool }
pub enum WindowEvent { Open, Focus(bool), AppearanceChanged{dark:bool}, Resize{points: Size, pixels: SizeU32},
    BackingScaleChanged{scale: f64, pixels: SizeU32}, KeyDown{code:u16, modifiers:Modifiers},
    KeyUp{code:u16, modifiers:Modifiers}, Pointer(PointerEvent), Scroll(ScrollEvent),
    Magnify{at:Point, amount:f64}, TextCommitted(String), Composition(Composition),
    InputReset{sequence:u64, all_keys_up:bool, all_buttons_up:bool}, CloseRequested,
    AnimationTick{monotonic_seconds:f64}, QueueFault(QueueFault) }
pub fn NativeApp::start(mt: MainThread) -> Result<Self, WindowError>;
pub fn NativeApp::create_window(&mut self, config: WindowConfig) -> Result<Window, WindowError>;
pub fn Window::id(&self) -> WindowId;
pub fn Window::set_title(&mut self, title: String) -> Result<(), WindowError>;
pub fn Window::set_visible(&mut self, visible: bool) -> Result<(), WindowError>;
pub fn Window::resize(&mut self, size: Size) -> Result<(), WindowError>;
pub fn Window::content_screen_origin(&self) -> Result<Point, WindowError>;
pub fn Window::set_cursor(&mut self, cursor: Cursor) -> Result<(), WindowError>;
pub fn Window::is_dark_appearance(&self) -> Result<bool, WindowError>;
pub fn Window::set_material_enabled(&mut self, enabled: bool) -> Result<bool, WindowError>;
pub fn Window::set_text_input_state(&mut self, state: TextInputState)
    -> Result<(), WindowError>;
pub fn Window::clear_text_input_state(&mut self) -> Result<(), WindowError>;
pub fn Window::acknowledge_input_reset(&mut self, sequence:u64) -> Result<(), WindowError>;
pub fn Window::request_ticks(&mut self, enabled: bool) -> Result<(), WindowError>;
pub fn NativeApp::poll_events(&mut self, window: WindowId, max: NonZeroUsize)
    -> Result<EventBatch, WindowError>;
pub fn NativeApp::destroy_window(&mut self, window: WindowId) -> Result<(), WindowError>;
pub fn NativeApp::shutdown(&mut self) -> Result<(), WindowError>;
```

`MainThread::acquire()` is the only safe acquisition path. `NativeApp` owns `Rc<RefCell<AppState>>`, registry and retained native records; each record exclusively owns `Retained<NSWindow>`, content/render views, delegate route, queue, timer and IME snapshot. `Window` owns only `Weak<RefCell<AppState>> + WindowId`, never native objects. AppState stores lifecycle (`Running|ShuttingDown|Stopped`), slot table and free slots; the main-thread capability is held by NativeApp. A slot generation is nonzero and increments before reuse; overflow retires the slot permanently. Stale slot generations return `ExpiredWindow`; an unavailable Weak owner or non-Running app returns `ExpiredOwner`. Every method upgrades/validates owner and generation before access. No RefCell/app-model borrow is held across an AppKit call that can reenter a callback. Once shutdown starts, mark expired and invalidate all routes/generations first, stop timers and detach callbacks, close/release every registered record on the main thread, then stop AppKit and release app state. `NativeApp::shutdown(&mut self)` and `Drop` share idempotent teardown. A surviving `Window` call after shutdown returns `ExpiredOwner`; its `Drop` only drops Weak and never creates/resurrects native owners or keeps AppKit running. `NativeApp`, `Window`, subscriptions, and route state are main-thread-only. Config title is nonempty valid Rust UTF-8 (no NUL), <=1,024 bytes; point dimensions 1..=4,096; coordinates/deltas finite and bounded. Event paths (only in later file gate) <=4,095 bytes. Each text snapshot/payload <=64 KiB; total pending owned payload <=1 MiB; queue capacity 256 events with 240 ordinary and 16 reserved control slots plus fixed-size sticky emergency latches. Checked arithmetic on point-to-pixel conversion; use rounded extents, never integer-truncate scale. Reject nonintegral or out-of-range backing scale until the accepted UI contract is extended (UI currently accepts integer 1..=3).

## File scope and ownership

Allowed production additions: `rust/crates/vexel-platform-apple/src/native_window/{mod.rs,types.rs,app.rs,window.rs,geometry.rs,input.rs,ime.rs,queue.rs,ticks.rs,teardown.rs}` and a single `mod native_window`/re-export addition to that crate's `src/lib.rs`. No manifest/lock changes. Allowed focused tests: `rust/crates/vexel-platform-apple/tests/native_window.rs` and private module unit tests colocated in these new files. Permit only paths already ignored by the repository under `.cache/` (including `.cache/tmp/` and local logs), `build/`, and `target/` for the named build/test outputs. Protected: every other Rust crate/source, all manifests/config/lockfiles, `src/`, headers, existing tests, ADR/docs/work orders/state/ledgers, existing artifacts, root contract work, system paths, sibling directories, and dependency downloads/installations. Keep each new production module <800 lines.

Map retained Cocoa owners with `Retained<T>`; create results own, getters remain borrowed unless retained before escape. App registry records exclusively own native window/view/delegate, queue, timer and IME snapshot. `Window` is only a weak app handle plus generation ID. Callback closures carry only `Weak<RouteState>` + `{slot,generation}` and bounded owned data. Invalidate app and each route generation first during teardown, then stop timer/wake source, cancel or detach callback sources, clear delegate/view routes, drain/release events and registry-owned native records, then stop AppKit. Stale callbacks/completions are ignored by generation. Do not retain native pointers in public values.

## Native behavior

**Startup/window:** `NativeApp::start` acquires shared `NSApplication`, regular activation policy, finish launch, and establishes main run-loop ownership. `create_window` validates all fields before native allocation; on any native failure, release partial objects and publish no handle. Use baseline window/style/backing APIs; set resize cap, responder/view routes and visibility/focus; queue initial `Open`, resize, and scale events before returning. Close delegate always returns false and queues `CloseRequested`; app code decides, then calls `NativeApp::destroy_window`. Do not silently equate close callback with destruction. Window handles can outlive shutdown safely because they contain only a Weak owner route; all methods check app lifecycle and slot generation before touching registry-owned native state.

**Appearance/material:** appearance queries/events use the baseline effective-appearance API. The legacy material effect is only enabled behind its existing macOS 26 availability check; `set_material_enabled` reports `Ok(false)` where unsupported (including macOS 13), without referencing the newer behavior on that runtime.

**Geometry:** represent logical positions in finite top-left content-view points. Convert AppKit bottom-left local events exactly once at the view boundary. Keep precise pointer/scroll coordinates and deltas as `f64`, integer logical coordinates as rounded/clamped compatibility values, backing pixel extents separate from points and `scale`. For screen origin, port current NSScreen/CGDisplayBounds conversion in `platform_window_geometry.m`; reject missing screen/display or non-finite result. Check all multiplications/additions and bounds before casts. Preserve scale-change even when pixel extent is unchanged. Unit-test 1x/2x/3x, negative screen origins, rounding edges, overflow, missing display, and nonfinite input.

**Input and IME:** translate NSEvent modifier bits explicitly to accepted `vexel-ui` ordering (UI shift=1, command=2, option=4, control=8; C order differs). Emit distinct key down/up, pointer down/up/move, scroll and magnify values; retain key code, button, click count, timestamps, precise deltas and scale. `keyDown` calls `interpretKeyEvents`; implement complete `NSTextInputClient`: `insertText`, `setMarkedText`, `unmarkText`, `hasMarkedText`, `markedRange`, `selectedRange`, `validAttributesForMarkedText`, `attributedSubstringForProposedRange:actualRange`, `firstRectForCharacterRange:actualRange`, `characterIndexForPoint`, and `doCommandBySelector`. App composition/focus code publishes an owned `TextInputState` through `Window::set_text_input_state`; callbacks read only the registry snapshot, never borrow/call the model. Require matching source `WindowId` generation; text <=64 KiB, selection/marked/attribute ranges ordered, in-bounds UTF-8 scalar boundaries, candidate rect finite and in content points; validate aggregate text+attribute payload before replacing the prior valid snapshot. UI publishes after every edit and focus change; explicit `clear_text_input_state` means no editable target. For an editable focused target, missing/stale snapshot latches `QueueFault::TextInputStateMissing` and returns an ABI-safe error/`NSNotFound` fallback; never fabricate empty text. `selectedRange` and `attributedSubstring` answer from the owned snapshot; marked attributes are bounded owned spans and only attributes from `validAttributesForMarkedText` are accepted. Convert every requested UTF-16 `NSRange` to UTF-8 and back with checked bounds/surrogate/scalar validation; invalid ranges latch `QueueFault::InvalidTextRange` and return explicit failure/`NSNotFound`, never clamp silently. `firstRectForCharacterRange` converts the saved candidate content-point rect to screen coordinates using the owning view/window generation. `characterIndexForPoint` performs the inverse conversion and returns a UTF-16 index only on valid scalar boundaries. Committed text is a separate owned event. Never infer committed text from `characters` or `charactersIgnoringModifiers`. UI `TextEdit` uses UTF-8 byte offsets; do conversion explicitly and preserve composition selection.

**Queue/ticks:** `EventBatch { events, pending }` is returned by poll; `pending` covers queued events, deferred native dispatch and sticky control latches. Capacity is 256 event records: 240 ordinary and 16 reserved controls, plus constant-size emergency fault/reset/close metadata with sequence numbers. Only adjacent tail pointer moves for the same window may coalesce; adjacent resize or scale snapshots of the same kind may replace each other. No cross-barrier search. No coalescing/dropping of key/button transitions, scroll, magnify, text/IME, focus, close, actions, or ticks. If no matching tail can replace incoming pointer move/resize/scale while ordinary capacity is exhausted, increment the explicit drop counter and emit/latch `QueueFault`; this is the only lossy input. Non-droppable input/control uses reserved slots. On control exhaustion, atomically stop ordinary ingress, sequence sticky `QueueFault` then `InputReset{sequence,all_keys_up:true,all_buttons_up:true}` after every already-accepted event, and preserve one sticky close request after reset; further releases are covered by the explicit reset and subsequent input remains disabled until the application acknowledges the delivered reset sequence with `acknowledge_input_reset`. Reject wrong/stale sequences without reopening input, preserve pending close, and keep controls serviceable while ordinary input is disabled. Close requests are idempotently latched and cannot be displaced by later events. `EventBatch` merges normal and sticky events by sequence, never reports `pending=false` while a latch/native event remains. No ignored enqueue failure. Poll drains at most caller's `max` (native dispatch itself at most 64 NSEvents per poll), returns whether more work remains, and lets caller continue without starving the run loop. Use one removable per-window NSTimer/CFRunLoop source and generation route, not the legacy global raw pointer; wake main run loop on scheduling/queue changes. Timer callbacks emit owned `AnimationTick` monotonic timestamps and never mutate UI/model state.

**Foreign boundary:** wrap all objc2 message sends and every inbound protocol/delegate/block callback with `guard_native_call`; catch Rust unwind and objc2 `catch-all` translated message exceptions, return ABI-valid fallback and latch orderly failure. No unwind crosses an ABI callback. This guard does not cover raw C/CF calls, retain/release, destructor cleanup, or `panic=abort`; avoid them in callbacks or document unavoidable process termination. Never claim a universal foreign exception fence. Do not use `new_unchecked` MainThreadMarker or add first-party ObjC exception shims.

## Explicit later gates (not part of this order)

1. **File/open-save/drop/reopen/recents:** `platform_window_files.m`, internal file dispatch, path/extension/startup queue rules, NSOpenPanel/NSSavePanel, async sheet cancellation, file URL/drop security-scoped lifetime and recents storage. Must define accepted owned document/job APIs first; no path payload in this lane.
2. **Menus/commands:** `platform_window_menus.m`, `platform_viewer_action`/command-state meaning, shortcut routing and state snapshots. Existing `CommandRegistry` is a seam, not complete menu parity.
3. **Accessibility:** full native AX tree/protocols, stable IDs/generations, screen-frame conversion, action routing and notifications over owned `SemanticsSnapshot`. UI snapshot alone is not VoiceOver support.
4. **Text shaping:** CoreText shaping/outlines and `TextDraw` adapter. This is not native text-input composition, already owned as an input contract above.
5. **Presenter:** Metal layer/device/queue/pipeline/upload/present/readback and resource lifetime. Window view here is not a rendering/presentation claim.
6. **Unsaved/dialogs:** dirty-document close/save/discard/cancel, export/error alerts, open/save panels and asynchronous completion cancellation.
7. **Preferences/appearance:** NSUserDefaults/system preference snapshots, removable subscriptions, theme/contrast/text-scale changes and notifications; basic effective-appearance event in this lane is not full preference parity.
8. **Welcome/recents:** welcome UI, recent-file persistence/refresh/open-failure state, path bounds and file-open integration.

## Acceptance evidence required when implemented

Focused tests: config limits/UTF-8/NUL; app-owned registry and retained-record teardown; generation reuse invalidating old handles; drop Window then use live app; shutdown/drop app while Window survives then call every operation and drop handle; expired-owner methods; drop cannot resurrect/retain app; geometry conversions and checked overflow; queue ordering/coalescing/barriers/capacity/reset/close under saturation; every key/button release survives or produces explicit reset; modifiers; timestamp monotonicity; UTF-16/UTF-8 IME ranges including surrogate pairs and replacement; snapshot generation/focus/selection/marked attributes/candidate conversion/missing-target fault; callback panic/objc exception fallbacks; timer cancellation; repeated shutdown and late callbacks. Build: macOS target `cargo check` for platform crate and focused test target, with existing pinned feature set only. Native boundary evidence: test harness/callback simulation without GUI launch, plus symbol/availability review against macOS 13 declarations. No test weakening.

The accepted binding/native-boundary decision and MainThread/guard_native_call APIs are prerequisites, not open adoption gates. Deferred manual/UI acceptance after implementation and downstream orders: launch on macOS 13 Apple Silicon; create/show/resize/move across displays and backing scales; close request/dirty decision; key press/release and pointer capture; real CJK/compose IME candidate placement and commit; wake/tick/stop under load; repeated shutdown with active callbacks; VoiceOver, menus, dialogs and actual presentation after their orders. Report exact host/OS and evidence. Planning work itself runs no build/test/GUI and makes no native runtime, macOS 13, or performance claim.

## Accepted prerequisite evidence (current, not the initial audit draft)

ADR0026 permits the pinned third-party exception helper/native build closure;
canonical grant evidence is retained under `artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01/coordinator-primary/canonical-licenses/` and the final integrated verification.
`WO-RUST-NATIVE-BINDINGS-SETUP-01` R2 compiled the selected16packages offline;
`WO-RUST-NATIVE-BOUNDARY-01` accepted5integration+2compile-fail checks including
real Foundation exception handling and cleanup termination. Current ledger
acceptance, manifests and those final artifacts supersede earlier pending-license
research text. No package/feature addition is authorized by this window order.
Release attribution/distribution and actual mac13runtime stay later gates.

# R2 coordinator feature seed and named completion repair
Initial Luna candidate R1 is preserved as incomplete. Coordinator may seed only
rust/crates/vexel-platform-apple/Cargo.toml and Cargo.lock from separately accepted
WO-RUST-NATIVE-WINDOW-FEATURE-01/ADR0027 into this isolated tree, plus this frozen
R2 order/ADR0027/pin supplement as read-only context. Worker must not modify these
files. Exact pins/active16/lock18 remain unchanged. Five existing binding features
support actual NSVisualEffectView, owned NSImage/NSBezierPath/NSGraphicsContext
24px diagonal and rotation cursors, and Foundation NSProcessInfo.

Sol6.1Medium named repair must complete every original window-lane requirement:
real NSView responder/NSWindowDelegate and NSTexInputClient callbacks, native event
pump with bounded pending delivery, actual NSTimer/run-loop wakeups for ticks,
Quartz global display geometry, complete cursor mapping including preserved
platform_cursor.m geometry/hotspots, appearance/resize/focus/backing/close callbacks,
validated owned IME snapshots and candidate/UTF16 queries, sequenced reset/fault
handling and safe weak generation teardown. Fix all RefCell borrows spanning
reentrant native calls and all partial publication/queue/control issues. Port
real material behind preserved macOS26 gate, not an always-error substitute.
Use original platform_window.m/platform_window_events.m/platform_cursor.m and
related headers/readiness maps as owned migration references. Read only named
files; source/tests<800. Existing original7 boundary tests byte-preserved. No
GUI launch/broadUI or fake tick/event/backend. Compile/focused callback/pure
proofs then independent Luna review and main integration checks. Standard existing
subscription only, no API billing/new deps. Product goal remains active.
