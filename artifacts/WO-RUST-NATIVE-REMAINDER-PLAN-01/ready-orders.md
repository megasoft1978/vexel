# Proposed complete native Rust implementation orders

These are planning scopes only. All production ownership, interfaces and tests remain unchanged until separate orders are accepted. Build `vexel-platform-apple` native implementations privately; public outputs are owned Rust values. No `lib.rs`, workspace manifest, lockfile, shared interface, or ledger edits belong to workers. Coordinator alone stages shared module exports/manifests/locks and freeze records after each gate. The accepted window source order supplies prerequisite APIs; it does not establish native runtime behavior.

## 0. NATIVE-MEDIA-TEXT

**Effort:** Luna Medium; CoreText shaping portion High. **Dependencies:** registered platform-owned DTO contract (coordinator gate below), existing `vexel-platform-apple` MainThread and `guard_native_call`; selected `objc2-core-text`, `objc2-core-graphics`, Foundation and ImageIO. Apple crate has no scene/UI/base dependency and does not gain one. **Owned source paths:** new `rust/crates/vexel-platform-apple/src/native_media/{text.rs,image.rs,gif.rs,tests.rs}` after coordinator registers module roots. A later root composition adapter in `rust/app/vexel-app/src/media_adapter.rs` implements `vexel_scene::MediaProvider`; it depends on scene and platform after coordinator workspace/dependency gate. **Contract:** platform DTOs `NativeTextRequest`, `NativeTextLayout { glyphs, commands, metrics }`, `NativeImage { rgba, byte_count, row_stride, width, height }`, `NativeAnimationFrame`, `NativeEncodedBytes`, and `NativeMediaError`; DTOs own results, borrow inputs only during call. CoreText returns actual shaped glyph advances and owned y-up outline commands; ImageIO returns actual top-row premultiplied RGBA static pixels; root adapter maps `WorkControl` and validates scene output. Root helper implements strict bounded PNG data URI and bilinear scaling; native GIF accepts borrowed frames during call and returns owned bytes. Preserve 64KiB text, 16,384 native glyph, 262,144 path segment, 32 families × 256 bytes, 32MiB input, 64M pixels, 16,384 image axis, exact PNG data URI prefix/44,739,266-byte URI bound, single-frame GIF decode/reject animation, 120 GIF frames, 4096 frame axis, 32,000,000 total frame pixels, 128MiB encoded output, fps ≤60 and all frame validation from source. Honor Rust scene's 65,536 output-glyph cap and 262,144 commands; stricter native 16,384 glyph limit applies. No substitute glyphs/pixels, filesystem or network reads. **Tests assigned:** exact canonical `platform_media_test.c` cases/assertion records and `media_probe.c` cases from `native-inventory.json` belong only to `NATIVE_MEDIA_TEXT`; fixture generator and README vectors are inputs/spec, not duplicate owners. Preserve malformed/truncated, limit edges, premultiplication/stride/scale, shaping/anchors/family/style/weights/color-only unsupported, URI 200k campaign, cancellation and owned result lifetime.

## 1. NATIVE-UI-TEXT-PREFERENCES

**Effort:** Luna Medium. **Dependencies:** order 0 for shared text shaping; current `vexel-ui::TextDraw` exact callback (in `src/widget_canvas.rs`) and `WidgetCanvas`; currently selected AppKit/CoreText/CoreGraphics/Foundation declarations, subject to every exact item-feature gate below. **Owned paths:** `native_text_ui/{draw.rs,preferences.rs,cache.rs,tests.rs}` only. Provide a main-thread-owned draw adapter that installs the actual callback and paints actual CoreText coverage into caller RGBA after checking dimensions, stride, buffer capacity and clip. No silent fallback if adapter is absent; return a surfaced draw failure. Implement observable current/extended preferences, app active state, UI scale accepted values 100/110/120/125/150/175/200 for menu where relevant, interface scale bounds 100..200, persisted panel widths and removable generation-safe observer subscriptions. Font rendering preserves 64KiB input/16,384 glyph bound, font px 1..256, render height ≤1024, cache 64 entries/4MiB, label normal style with weights 400/600, UI label/tree bounds (128 nodes/depth 24, 63-byte previews, 1MiB total strings, draft 127 bytes/label 63 bytes). `TextDraw` only receives its established parameters; use no fabricated fonts or pixels. **Tests:** the sole-owner platform text UI cases in `tests/ui/platform_text_ui_test.c`; `tests/platform_media/platform_media_test.c` remains owned solely by media lane0 and `tests/ui/native_ui_render.m` solely by composition lane7; cache eviction/bytes, clipping/stride/capacity, all scale/preferences validation, observer removal/late callbacks, text fitting/layout and actual glyph raster evidence.

## 2. NATIVE-PRESENTER-SHADERS

**Effort:** Luna High. **Dependencies:** accepted window native shell view/lifecycle; `vexel-scene::ImageView` or exact `RgbaFrame` ownership contract after coordinator pins it; selected Metal/QuartzCore/CoreGraphics bindings. **Owned paths:** `native_present/{mod.rs,presenter.rs,shaders.rs,tests.rs}` plus owned `native_present/shaders/*.metal` resource files. Define per-window `Presenter` with checked borrowed-frame copy before returning, per-window reusable textures and pipeline, bounded in-flight GPU submissions, explicit pixel format/channel/alpha contract, recoverable no-drawable status, completion generation routing and teardown drain. Keep separate synchronous diagnostic readback/hash API only if legacy validation requires it; no global device resources coupled to a C window pointer. Preserve the actual PNG/Metal 4096-axis checks, PNG 16,000,000-pixel and 64MiB encoded-byte caps, and checked width×height×4/byte-count contracts; five-second diagnostic readback deadline where existing source specifies it. **Tests:** original tile/metal presentation tests and `platform_metal_present_and_verify`/`platform_metal_copy_and_hash` cases; exact bytes/readback hashes, device/layer unavailable, drawable missing, dimensions/overflow, failure/timeout, multiple windows isolation, asynchronous completion after reset/drop, resource lifetime and shader compile on supported macOS SDK.

## 3. NATIVE-MENUS-COMMANDS

**Effort:** Luna Medium. **Dependencies:** accepted window `WindowId`, owned command registry/state snapshot and command dispatch seam; current selected AppKit NSMenu/NSMenuItem; localization order/interface. **Owned paths:** `native_menus/{mod.rs,menus.rs,commands.rs,tests.rs}` only. Port the entire menu tree, separators, shortcut discovery, recent-item submenus (the actual 8-entry `WELCOME_RECENT_LIMIT` model and menu loop; README says 12 and is a recorded documentation discrepancy), state enabling/checkmarks, full-screen toggles, option state, ruler context menus and command callbacks. Menus consume immutable owned state and emit exactly one typed command routed by live window generation. Preserve all `platform_viewer_action` IDs and modifier mapping; don't equate menu construction with app command implementation. **Tests:** `tests/app/platform_command_policy_test.c`, original menu test hooks/source assertions; native-shell render cases belong solely to composition lane7; cover all command IDs and check states, shortcut modifiers, stale generation, recent refresh, context position conversion, localization and callback faults.

## 4. NATIVE-DIALOGS-FILES-DROP

**Effort:** Luna High. **Dependencies:** accepted Rust document IO/save/job contracts and window generation/late callback API; selected NSOpenPanel/NSSavePanel/NSAlert/NSDragging/NSPasteboard; existing `vexel-platform-tasks`. **Owned paths:** `native_files/{mod.rs,open.rs,save.rs,drop.rs,clipboard.rs,dialogs.rs,tests.rs}` only. Implement open/save/export panels and unsaved/recovery/error dialogs including asynchronous completion cancellation; open URL/startup files, security-scoped lifetime, SVG-only drop rules, clipboard SVG/text/PNG and startup queue. Convert selected paths to owned validated UTF-8 strings; no borrowed C buffers. Preserve window path 4096 cap, dialog path 1024/text 4096, startup 8 entries, clipboard SVG 64KiB and existing extension/filter/overwrite rules; reject or explicitly report overflow. All completions carry window and operation generation; cancel/invalidate before owner teardown. **Tests:** `tests/app/project_save_test.c`, dialogs/platform window file tests, `tests/editor/editor_clipboard_ids_test.c`, clipboard tests, platform save/open test hooks and file fixtures. Cover cancel/accept/error, extension rules, invalid UTF8/long paths, drop enter/exit/perform filtering, startup saturation, scoped URL release, cancellation and stale completions.

## 5. NATIVE-ACCESSIBILITY

**Effort:** Luna High. **Dependencies:** accepted `vexel-ui` owned `SemanticsSnapshot`/tree contracts and live window generation; selected AX element/protocol/constants and NSView. **Owned paths:** `native_accessibility/{mod.rs,tree.rs,actions.rs,tests.rs}` only. Build a real native AX tree from snapshots, preserve IDs/roles/labels/value/actions/selection/visibility, convert pixel frames to screen coordinates with backing-scale and origin semantics, route actions through owned commands, post value/layout/focus notifications and safely unregister callbacks. No claim that the Rust snapshot itself supplies VoiceOver. Preserve 128 nodes and existing text/identifier caps; reject malformed snapshots without partially replacing prior accessible tree. **Tests:** `tests/editor/accessibility_localization_test.c`, `tests/editor/inspector_tab_reachability_test.c`, the exact AX/localization checker records in the R3 inventory; native_ui_render has no invented AX section and its render cases belong solely to composition lane7; stable identity, tree replacement, hidden panels, frame conversion, action routing/staleness, notifications and teardown.

## 6. NATIVE-SETTINGS-RECENTS-WELCOME-LOCALIZATION

**Effort:** Luna Medium. **Dependencies:** settings UI model and accepted window/file/menus interfaces; selected NSUserDefaults, NSWorkspace, Foundation/AppKit; localization table/tooling contract. **Owned paths:** `native_preferences/{mod.rs,settings.rs,recents.rs,welcome.rs,localization.rs,feedback.rs,tests.rs}` only. Port preference read/write/default seeding, workspace/sidebar/density, recent path persistence/availability/open-failure, welcome surface actions, localized labels/numbers/plural messages and feedback/help. Use removable subscriptions and copied snapshots. Preserve 16-byte workspace field, 8 entries in `platform_window.h` event API; welcome/recents use `WELCOME_RECENT_PATH_CAP` (4096). Use the actual menu loop and welcome model cap of 8 entries; README says up to 12 and is not implementation proof. Do not adopt 12 or infer a display-name cap not declared by the active structure; localization 128-byte keys/4096-byte text, welcome recent path cap from `welcome_model.h`; settings must retain test-domain isolation semantics. Welcome is custom Rust UI, not native system substitute. **Tests:** `tests/app/viewer_home_test.c`, `welcome_model_test.c`, `viewer_home_layout_fuzz.c`, settings tests, `tests/editor/accessibility_localization_test.c`, localization extraction/pseudo-locale checks and feedback diagnostics tests.

## 7. APP-COMPOSITION

**Effort:** Luna High. **Dependencies:** accepted complete orders 0–6, window source gate, Rust document/model/render/timeline contracts. **Owned paths:** new `rust/app/vexel-app/**` app composition modules and executable source only; coordinator owns workspace membership, `vexel-platform-apple/src/lib.rs`, app crate manifest/resources/lock and all shared integration. Compose lifecycle, document open/save/cancel generations, media provider, custom UI `WidgetCanvas` with concrete `TextDraw`, menus, accessibility, Metal `Presenter`, preferences/welcome, recents, clipboard/drop and dialogs. Every adapter must be installed and failure observable; no fake defaults, legacy C backend, hidden fallback, or incomplete-path claim. **Tests/evidence:** migrate only the individually composition-owned original cases in the R3 ownership map; consume native-lane parity results without duplicating their ownership or weakening fixtures/oracles; headless composition tests first, then broad macOS Apple Silicon GUI/E2E on minimum macOS 13 and current host covering launch, document lifecycle, text/image/GIF, IME, scale/display changes, menus/dialogs, VoiceOver, drop/clipboard, errors, cancellation, teardown and authentic character-animation demonstration. These are future gates, not planning results.

## Coordinator gates and shared proof

Before any module worker starts, coordinator accepts the safe DTO/work-budget contract, registers `native_media`, `native_text_ui`, `native_present`, `native_menus`, `native_files`, `native_accessibility`, and `native_preferences` in `vexel-platform-apple/src/lib.rs`, and lands compile-visible module roots. Separately coordinator adds the root app crate/workspace member and adapter modules only after its acyclic dependency review. This registration is not worker implementation and no unregistered body is claimed to compile. Workers may edit only their registered private module/test paths. Coordinator exclusively edits `lib.rs`, Cargo manifests/lock, workspace membership, shared interfaces, ledgers and freeze evidence. Each worker handoff is followed by registered-target compile/test and independent source/hash coverage review before integration.

## Shared implementation verification and freeze gates

The R3 inventory below already names full original file/hash coverage, each public/private function and helper, original cap/check anchors, and sole ownership of every recorded individual case/assertion. Separate implementation orders must preserve those exact assignments, add exact-edge and ownership/late-callback tests, and retain MPL-2.0/macOS-13/Apple-Silicon/custom-UI/owned-SVG restrictions. Evidence must identify exact Rust test names and fixture provenance; focused locked/offline target check and tests, API/feature review, per-callback ABI guard audit, main-thread proof, CF/ObjC ownership audit, allocation/cancellation/teardown review, output path/hash audit and independent review are required. macOS 13 runtime and broad GUI evidence remain separate release gates. No build/test was run for this proposal.

# Revision 3 exact proof supplement

This completes the SAME frozen revision-3 planning scope, not a new implementation plan. The R2 source/method/block records and all eight whole lanes above are retained. `native-inventory.json` R3 sections supersede broad R2 test-path assignments and prose limit summaries. [api-proof.md](api-proof.md) contains every API record; [original-case-ownership.md](original-case-ownership.md) contains every individual original case/check. Full expressions, function bodies, exact file/record hashes, SDK signatures and Cargo declarations remain machine-readable in the inventory. No source/build/runtime acceptance follows from these reports. A blocked metadata row is an explicit pre-worker coordinator gate; an implementation worker is not asked to find the missing evidence later.

## Coordinator-owned safe adapter and internal metadata gate (no adoption)

The production Apple crate has no scene/UI/base dependency. Its current `Cargo.toml` and all accepted `NativeApp`/`Window`/`WindowId`/private `NativeView` declarations remain unchanged. The following is a concrete **proposed contract**, not an already registered API or an enabled feature. Coordinator alone reviews/registers `native_adapter_contract/{mod.rs,media.rs,ui.rs,window_target.rs}` and module roots in `vexel-platform-apple/src/lib.rs`; these coordinator paths are excluded from every implementation worker. Coordinator alone registers the root app crate, its manifest/workspace/lock entries, and shared adapter interfaces. No new package is proposed here.

```rust
// Proposed platform-owned types; no imports from scene/UI/base.
pub enum NativeStop { Canceled, Limit, Overflow }
pub trait NativeControl {
    fn checkpoint(&mut self) -> Result<(), NativeStop>;
    fn charge_media(&mut self, units: u64) -> Result<(), NativeStop>;
}
pub enum NativeTextStyle { Normal, Italic, Oblique }
pub struct NativeTextRequest<'a> {
    pub text: &'a [u8], pub families: &'a [&'a [u8]],
    pub size_points: f64, pub weight: f64,
    pub style: NativeTextStyle, pub letter_spacing_points: f64,
}
pub enum NativePathVerb { Move, Line, Cubic, Close }
pub struct NativePathCommand { pub verb: NativePathVerb, pub values: [f64; 6] }
pub struct NativeGlyph {
    pub id: u16, pub x: f64, pub y: f64, pub advance: f64,
    pub command_start: usize, pub command_count: usize,
}
pub struct NativeTextLayout {
    pub glyphs: Vec<NativeGlyph>, pub commands: Vec<NativePathCommand>,
    pub advance_width: f64, pub ascent: f64, pub descent: f64, pub leading: f64,
}
pub struct NativeImage {
    pub rgba: Vec<u8>, pub byte_count: usize, pub row_stride: usize,
    pub width: u32, pub height: u32,
}
pub struct NativeFrame<'a> {
    pub rgba: &'a [u8], pub row_stride: usize, pub width: u32, pub height: u32,
}
pub enum NativeMediaError {
    Unsupported, InvalidArgument, InvalidUtf8, Animated, Corrupt,
    Limit, OutOfMemory, Canceled, Overflow,
}
// Enum discriminants map existing original command/role/state values explicitly.
pub struct NativeCommandId(pub u32);
pub struct NativeRect { pub x: f64, pub y: f64, pub width: f64, pub height: f64 }
pub struct NativeSemanticsNode {
    pub id: u32, pub parent_id: u32, pub role: u32, pub state: u32,
    pub command: NativeCommandId, pub label: String, pub value: String,
    pub frame: NativeRect,
}
pub struct NativeSemantics { pub nodes: Vec<NativeSemanticsNode> }
pub struct NativeOperationToken {
    pub window: WindowId, pub operation_generation: u64,
}
```

`NativeMedia` proposed operations are `layout_text(&mut self, NativeTextRequest<'_>, &mut dyn NativeControl) -> Result<NativeTextLayout, NativeMediaError>`, `decode_image(&mut self, &[u8], &mut dyn NativeControl) -> Result<NativeImage, NativeMediaError>`, `encode_gif(&mut self, &[NativeFrame<'_>], fps: u32, &mut dyn NativeControl) -> Result<Vec<u8>, NativeMediaError>`. Inputs are borrowed only for the call; all output is owned. Checked allocation/arithmetic and every original bound apply before FFI and before publishing. Control charges/checkpoints occur in batches of at most 256 units; bounded indivisible CoreText/ImageIO calls are checkpointed immediately before/after and canceled results are discarded. This proposes no cancellation of an arbitrary in-progress native call.

`MediaAdapter` in `rust/app/vexel-app/src/media_adapter.rs` implements the **actual** `vexel_scene::MediaProvider` signatures at `vexel-scene/src/media.rs:118` using the above DTOs. It maps glyph IDs to `Some(id)`, checks every command range, preserves y-up outlines, validates row stride/byte count/top-row premultiplied RGBA and uses scene result validation. A root `ControlAdapter<'a>` borrows `&mut dyn vexel_base::WorkControl`, charges `WorkKind::Media`, and retains the first `Stop` in root-owned operation state. Existing `MediaError` lacks cancellation variants: the composition caller must inspect that retained stop before interpreting a provider error, so cancellation/overflow are never erased into a generic image error. This precise error-path contract is a coordinator gate; no base dependency is added to Apple.

`TextAdapter` in `rust/app/vexel-app/src/text_adapter.rs` supplies the exact `vexel_ui::TextDraw` callback from `widget_canvas.rs:14`. It maps UI enums/rectangles into platform-owned arguments and a checked caller-owned `&mut [u8]`; platform coverage/cache methods return `bool`/owned platform error. Root tracks a failed callback and surfaces the rendering failure. Every production `WidgetCanvas` gets the callback. Native accepts interface text scale 110/120 whereas current `WidgetCanvas::set_text_scale` accepts only 25-point steps: the coordinator must accept explicit adapter/layout metadata for those two values before this lane starts; do not round, reject previously accepted values, or silently change the shared UI interface. The 1MiB UI string storage cap does not enlarge the native 64KiB per-label shaping limit.

`SemanticsAdapter` in `rust/app/vexel-app/src/semantics_adapter.rs` translates the actual `vexel_ui::SemanticsSnapshot` (`interaction.rs:24`) into owned platform DTOs. Validate 128 nodes, parent/ID uniqueness, known roles/state/action IDs, finite coordinate transforms and 1MiB full strings; preview strings do not replace full labels/values. Apple never imports `vexel_ui`. A native tree swap is atomic after full validation and generation check.

`Presenter::attach(&Window, MainThread)`/`present(&Window, NativeFrame<'_>, NativeOperationToken)` use the existing crate-private `Window::surface_view` (`native_window/window.rs:226`). No public native handle or change to accepted window methods is proposed. `NativeView` stays private; remaining modules never subclass a second application view or access a fabricated raw pointer. The root app owns a manager keyed by `WindowId { slot, generation }` and operation generation, passes a live `&Window` on each operation, invalidates its own completion token before detaching/destroying and drops presenters before `NativeApp::destroy_window`. GPU callbacks retain only owned completion data and a weak/alive token; they cannot borrow app/model/window state. The coordinator reviews a concrete maximum of 3 in-flight submissions as a new policy bound (not a falsely inherited limit). Present is asynchronous; legacy diagnostic readback remains separately bounded to 5 seconds.

A proposed `native_present/completion.rs` owns a bounded Rust completion channel and deadline; `native_present/digest.rs` owns `fn sha256(bytes: &[u8]) -> [u8; 32]` for the preserved readback/replay oracle. These are explicitly gated first-party adapter contracts, not a dispatch2/CommonCrypto package or raw-extern adoption. Exact GPU completion APIs already have cached/SDK proof. Full implementations and original byte/hash assertions remain the presenter's scope.

Menus/dialogs/drop/AX/preferences receive copied platform state and emit `NativeCommandId` plus `NativeOperationToken`; root translates to existing editor/document commands. No model borrow crosses a native call. Callbacks and public native operations enter `guard_native_call`; main-thread ownership, CF/ObjC retain rules, cancellation, observer removal, completion invalidation and teardown ordering remain mandatory. No blanket claim that destructors or raw CF faults can be recovered is made.

The metadata gate also reviews the exact `R3-API-METADATA-*` proposals in the inventory. Each proposal contains its exact source occurrences, nominated cached declaration when present, required selected-feature delta, SDK availability evidence and coordinator action. These are review-only requests, not permission to adopt dependencies/features. Unselected native control/NSFont declarations do not become available by choosing a feature name; custom welcome/settings/help/feedback surfaces must preserve behavior via owned custom UI and real CoreText. Shared registration occurs only after the contract/metadata review. All actual worker orders remain to be frozen independently by the coordinator.

## Full-lane proof index

The following rows cover every lane above. API IDs enumerate the complete corresponding records, not representative examples. Test and numeric counts refer to individual, sole-owner records with original path/function/line/SHA256. Some API identities are shared between lanes; their **source uses** remain separately anchored. Tests/assertions are never duplicated between owners.

| Whole lane | Source files | Definitions | API rows | Pre-worker metadata gates | Individual test records | Numeric/source/test anchors |
|---|---:|---:|---:|---:|---:|---:|
| NATIVE_MEDIA_TEXT | 9 | 28 | 103 | 4 | 256 | 576 |
| NATIVE_UI_TEXT_PREFERENCES | 2 | 31 | 68 | 9 | 19 | 151 |
| NATIVE_PRESENTER | 5 | 12 | 85 | 10 | 320 | 82 |
| NATIVE_MENUS | 2 | 33 | 75 | 11 | 19 | 42 |
| NATIVE_FILES_DIALOGS | 5 | 25 | 61 | 18 | 217 | 296 |
| NATIVE_ACCESSIBILITY | 2 | 29 | 85 | 7 | 99 | 120 |
| NATIVE_WELCOME_LOCALIZATION | 11 | 97 | 226 | 100 | 148 | 296 |
| APP_COMPOSITION | 17 | 111 | 185 | 28 | 6590 | 14191 |

### NATIVE_MEDIA_TEXT complete assignments

Original native files: `src/platform/media/README.md`, `src/platform/media/platform_animation.h`, `src/platform/media/platform_animation.m`, `src/platform/media/platform_image.h`, `src/platform/media/platform_image.m`, `src/platform/media/platform_media.c`, `src/platform/media/platform_media.h`, `src/platform/media/platform_text.h`, `src/platform/media/platform_text.m`.

All API records: API-0002, API-0003, API-0004, API-0005, API-0006, API-0007, API-0008, API-0009, API-0010, API-0011, API-0013, API-0014, API-0015, API-0016, API-0017, API-0018, API-0019, API-0020, API-0021, API-0022, API-0023, API-0024, API-0025, API-0026, API-0027, API-0028, API-0029, API-0031, API-0034, API-0035, API-0036, API-0038, API-0040, API-0042, API-0046, API-0047, API-0049, API-0050, API-0051, API-0052, API-0053, API-0054, API-0055, API-0056, API-0057, API-0058, API-0059, API-0060, API-0061, API-0064, API-0069, API-0072, API-0074, API-0075, API-0076, API-0077, API-0078, API-0079, API-0080, API-0081, API-0082, API-0083, API-0084, API-0085, API-0086, API-0087, API-0088, API-0364, API-0365, API-0366, API-0367, API-0368, API-0369, API-0370, API-0371, API-0372, API-0373, API-0374, API-0375, API-0376, API-0378, API-0379, API-0380, API-0381, API-0382, API-0383, API-0384, API-0385, API-0386, API-0387, API-0388, API-0389, API-0390, API-0391, API-0392, API-0393, API-0394, API-0395, API-0396, API-0397, API-0398, API-0399, API-0400.

Coordinator prerequisites: R3-API-METADATA-0072, R3-API-METADATA-0394, R3-API-METADATA-0396, R3-API-METADATA-0398.

All referenced native enum/constant records: CONST-0082, CONST-0083, CONST-0084, CONST-0085, CONST-0086, CONST-0087, CONST-0088, CONST-0089, CONST-0090, CONST-0091, CONST-0092, CONST-0093, CONST-0095, CONST-0096, CONST-0097, CONST-0098, CONST-0099, CONST-0100, CONST-0101, CONST-0102, CONST-0103, CONST-0104, CONST-0105, CONST-0106, CONST-0107, CONST-0108, CONST-0109, CONST-0110, CONST-0111, CONST-0112, CONST-0113, CONST-0114, CONST-0115, CONST-0116.

Additional exact constant metadata gates: R3-CONSTANT-METADATA-0112, R3-CONSTANT-METADATA-0114.

### NATIVE_UI_TEXT_PREFERENCES complete assignments

Original native files: `src/platform/platform_text_ui.h`, `src/platform/platform_text_ui.m`.

All API records: API-0030, API-0031, API-0033, API-0036, API-0037, API-0039, API-0040, API-0041, API-0043, API-0044, API-0045, API-0062, API-0063, API-0065, API-0066, API-0067, API-0068, API-0069, API-0114, API-0115, API-0116, API-0123, API-0145, API-0146, API-0147, API-0168, API-0169, API-0170, API-0173, API-0191, API-0209, API-0223, API-0231, API-0236, API-0250, API-0258, API-0261, API-0278, API-0295, API-0313, API-0316, API-0325, API-0334, API-0351, API-0357, API-0358, API-0377, API-0406, API-0429, API-0456, API-0457, API-0458, API-0459, API-0460, API-0461, API-0462, API-0496, API-0499, API-0500, API-0501, API-0516, API-0587, API-0588, API-0648, API-0649, API-0660, API-0702, API-0703.

Coordinator prerequisites: R3-API-METADATA-0030, R3-API-METADATA-0168, R3-API-METADATA-0191, R3-API-METADATA-0236, R3-API-METADATA-0313, R3-API-METADATA-0316, R3-API-METADATA-0334, R3-API-METADATA-0460, R3-API-METADATA-0501.

All referenced native enum/constant records: CONST-0027, CONST-0028, CONST-0065, CONST-0068, CONST-0075, CONST-0079, CONST-0094.

Additional exact constant metadata gates: none.

### NATIVE_PRESENTER complete assignments

Original native files: `src/platform/macos/platform_metal.m`, `src/platform/macos/platform_metal_internal.h`, `src/platform/macos/platform_png.h`, `src/platform/macos/platform_png.m`, `src/platform/platform_metal.h`.

All API records: API-0001, API-0010, API-0012, API-0013, API-0023, API-0024, API-0025, API-0032, API-0035, API-0036, API-0046, API-0047, API-0049, API-0050, API-0051, API-0052, API-0056, API-0073, API-0089, API-0090, API-0091, API-0092, API-0163, API-0174, API-0206, API-0213, API-0220, API-0232, API-0240, API-0248, API-0249, API-0252, API-0253, API-0269, API-0284, API-0285, API-0286, API-0287, API-0293, API-0300, API-0301, API-0312, API-0320, API-0356, API-0358, API-0364, API-0374, API-0376, API-0378, API-0390, API-0409, API-0410, API-0448, API-0450, API-0451, API-0452, API-0453, API-0454, API-0480, API-0482, API-0504, API-0505, API-0506, API-0546, API-0558, API-0563, API-0566, API-0572, API-0573, API-0574, API-0578, API-0593, API-0608, API-0611, API-0613, API-0621, API-0622, API-0641, API-0642, API-0644, API-0645, API-0663, API-0689, API-0690, API-0707.

Coordinator prerequisites: R3-API-METADATA-0001, R3-API-METADATA-0073, R3-API-METADATA-0206, R3-API-METADATA-0284, R3-API-METADATA-0285, R3-API-METADATA-0286, R3-API-METADATA-0287, R3-API-METADATA-0300, R3-API-METADATA-0301, R3-API-METADATA-0320.

All referenced native enum/constant records: CONST-0001, CONST-0002, CONST-0003, CONST-0004, CONST-0005, CONST-0006, CONST-0007, CONST-0082, CONST-0093, CONST-0095, CONST-0107.

Additional exact constant metadata gates: none.

### NATIVE_MENUS complete assignments

Original native files: `src/platform/macos/platform_menus.m`, `src/platform/macos/platform_window_menus.m`.

All API records: API-0119, API-0122, API-0135, API-0156, API-0157, API-0164, API-0178, API-0179, API-0185, API-0191, API-0194, API-0195, API-0196, API-0197, API-0219, API-0235, API-0242, API-0268, API-0283, API-0299, API-0304, API-0305, API-0318, API-0323, API-0324, API-0339, API-0340, API-0352, API-0357, API-0358, API-0360, API-0361, API-0362, API-0363, API-0408, API-0414, API-0426, API-0427, API-0428, API-0434, API-0435, API-0463, API-0464, API-0468, API-0471, API-0473, API-0481, API-0495, API-0507, API-0532, API-0560, API-0561, API-0569, API-0580, API-0591, API-0598, API-0614, API-0617, API-0619, API-0626, API-0630, API-0631, API-0633, API-0646, API-0653, API-0656, API-0657, API-0665, API-0678, API-0679, API-0681, API-0682, API-0688, API-0691, API-0716.

Coordinator prerequisites: R3-API-METADATA-0191, R3-API-METADATA-0304, R3-API-METADATA-0305, R3-API-METADATA-0323, R3-API-METADATA-0362, R3-API-METADATA-0363, R3-API-METADATA-0408, R3-API-METADATA-0471, R3-API-METADATA-0619, R3-API-METADATA-0665, R3-API-METADATA-0678.

All referenced native enum/constant records: CONST-0030, CONST-0031, CONST-0037, CONST-0038, CONST-0067.

Additional exact constant metadata gates: R3-CONSTANT-METADATA-0037, R3-CONSTANT-METADATA-0038.

### NATIVE_FILES_DIALOGS complete assignments

Original native files: `src/platform/macos/platform_clipboard.h`, `src/platform/macos/platform_clipboard.m`, `src/platform/macos/platform_dialog.h`, `src/platform/macos/platform_dialog.m`, `src/platform/macos/platform_window_files.m`.

All API records: API-0123, API-0131, API-0132, API-0135, API-0137, API-0139, API-0162, API-0165, API-0171, API-0175, API-0181, API-0185, API-0188, API-0204, API-0224, API-0231, API-0241, API-0274, API-0275, API-0277, API-0311, API-0317, API-0321, API-0324, API-0332, API-0335, API-0350, API-0351, API-0353, API-0358, API-0407, API-0408, API-0413, API-0414, API-0421, API-0472, API-0474, API-0475, API-0514, API-0515, API-0517, API-0527, API-0537, API-0539, API-0543, API-0554, API-0559, API-0575, API-0576, API-0582, API-0602, API-0604, API-0623, API-0662, API-0668, API-0669, API-0673, API-0676, API-0678, API-0699, API-0718.

Coordinator prerequisites: R3-API-METADATA-0165, R3-API-METADATA-0181, R3-API-METADATA-0317, R3-API-METADATA-0321, R3-API-METADATA-0335, R3-API-METADATA-0407, R3-API-METADATA-0408, R3-API-METADATA-0472, R3-API-METADATA-0474, R3-API-METADATA-0475, R3-API-METADATA-0527, R3-API-METADATA-0543, R3-API-METADATA-0662, R3-API-METADATA-0668, R3-API-METADATA-0669, R3-API-METADATA-0673, R3-API-METADATA-0678, R3-API-METADATA-0699.

All referenced native enum/constant records: CONST-0023, CONST-0024, CONST-0025, CONST-0026, CONST-0038, CONST-0056, CONST-0059, CONST-0060, CONST-0067.

Additional exact constant metadata gates: R3-CONSTANT-METADATA-0038.

### NATIVE_ACCESSIBILITY complete assignments

Original native files: `src/platform/macos/platform_accessibility.h`, `src/platform/macos/platform_accessibility.m`.

All API records: API-0093, API-0094, API-0095, API-0096, API-0097, API-0098, API-0099, API-0100, API-0101, API-0102, API-0103, API-0104, API-0105, API-0106, API-0107, API-0108, API-0109, API-0110, API-0111, API-0112, API-0113, API-0119, API-0122, API-0123, API-0125, API-0127, API-0128, API-0135, API-0143, API-0144, API-0148, API-0149, API-0150, API-0151, API-0152, API-0153, API-0154, API-0155, API-0164, API-0167, API-0179, API-0185, API-0191, API-0198, API-0199, API-0258, API-0259, API-0263, API-0264, API-0265, API-0266, API-0267, API-0281, API-0282, API-0323, API-0324, API-0340, API-0344, API-0345, API-0346, API-0347, API-0348, API-0350, API-0358, API-0363, API-0406, API-0414, API-0427, API-0441, API-0447, API-0477, API-0478, API-0483, API-0529, API-0532, API-0535, API-0536, API-0630, API-0631, API-0678, API-0682, API-0686, API-0692, API-0694, API-0713.

Coordinator prerequisites: R3-API-METADATA-0191, R3-API-METADATA-0259, R3-API-METADATA-0267, R3-API-METADATA-0323, R3-API-METADATA-0363, R3-API-METADATA-0447, R3-API-METADATA-0678.

All referenced native enum/constant records: CONST-0008, CONST-0009, CONST-0010, CONST-0011, CONST-0012, CONST-0013, CONST-0014, CONST-0015, CONST-0016, CONST-0017, CONST-0018, CONST-0019, CONST-0020, CONST-0021, CONST-0022, CONST-0037, CONST-0038, CONST-0042, CONST-0043, CONST-0067.

Additional exact constant metadata gates: R3-CONSTANT-METADATA-0037, R3-CONSTANT-METADATA-0038.

### NATIVE_WELCOME_LOCALIZATION complete assignments

Original native files: `src/platform/macos/platform_feedback.m`, `src/platform/macos/platform_localization.m`, `src/platform/macos/platform_recents.m`, `src/platform/macos/platform_settings.m`, `src/platform/macos/platform_welcome.m`, `src/platform/platform_feedback.h`, `src/platform/platform_localization.h`, `src/platform/platform_localization_defaults.c`, `src/platform/platform_localization_internal.h`, `src/platform/platform_recents.h`, `src/platform/platform_settings.h`.

All API records: API-0118, API-0120, API-0121, API-0123, API-0124, API-0130, API-0132, API-0134, API-0135, API-0138, API-0140, API-0141, API-0158, API-0159, API-0160, API-0161, API-0162, API-0166, API-0167, API-0171, API-0172, API-0175, API-0180, API-0182, API-0183, API-0184, API-0185, API-0186, API-0187, API-0188, API-0190, API-0191, API-0199, API-0200, API-0203, API-0207, API-0211, API-0214, API-0215, API-0217, API-0218, API-0223, API-0224, API-0225, API-0226, API-0227, API-0228, API-0229, API-0231, API-0233, API-0234, API-0237, API-0238, API-0239, API-0241, API-0242, API-0243, API-0251, API-0254, API-0255, API-0256, API-0257, API-0260, API-0268, API-0270, API-0272, API-0273, API-0275, API-0279, API-0289, API-0290, API-0294, API-0303, API-0304, API-0305, API-0306, API-0307, API-0311, API-0315, API-0318, API-0319, API-0321, API-0322, API-0323, API-0324, API-0325, API-0327, API-0328, API-0329, API-0330, API-0331, API-0332, API-0334, API-0338, API-0341, API-0349, API-0350, API-0352, API-0354, API-0355, API-0357, API-0358, API-0359, API-0406, API-0407, API-0408, API-0411, API-0412, API-0413, API-0414, API-0416, API-0417, API-0426, API-0430, API-0432, API-0433, API-0437, API-0449, API-0459, API-0464, API-0467, API-0468, API-0469, API-0470, API-0473, API-0476, API-0479, API-0483, API-0485, API-0486, API-0487, API-0488, API-0489, API-0490, API-0491, API-0492, API-0493, API-0494, API-0497, API-0498, API-0502, API-0503, API-0508, API-0511, API-0514, API-0526, API-0528, API-0530, API-0531, API-0540, API-0541, API-0548, API-0550, API-0551, API-0564, API-0565, API-0567, API-0568, API-0570, API-0571, API-0577, API-0579, API-0582, API-0587, API-0589, API-0590, API-0592, API-0594, API-0595, API-0596, API-0597, API-0600, API-0601, API-0602, API-0603, API-0605, API-0606, API-0607, API-0609, API-0616, API-0618, API-0623, API-0624, API-0625, API-0629, API-0632, API-0635, API-0636, API-0637, API-0638, API-0639, API-0640, API-0643, API-0647, API-0650, API-0651, API-0652, API-0654, API-0655, API-0659, API-0661, API-0662, API-0666, API-0667, API-0670, API-0671, API-0672, API-0675, API-0677, API-0678, API-0679, API-0680, API-0683, API-0684, API-0685, API-0687, API-0693, API-0695, API-0696, API-0697, API-0698, API-0705, API-0706, API-0714, API-0715, API-0717.

Coordinator prerequisites: R3-API-METADATA-0120, R3-API-METADATA-0134, R3-API-METADATA-0140, R3-API-METADATA-0141, R3-API-METADATA-0158, R3-API-METADATA-0160, R3-API-METADATA-0161, R3-API-METADATA-0180, R3-API-METADATA-0182, R3-API-METADATA-0183, R3-API-METADATA-0184, R3-API-METADATA-0186, R3-API-METADATA-0191, R3-API-METADATA-0211, R3-API-METADATA-0215, R3-API-METADATA-0217, R3-API-METADATA-0218, R3-API-METADATA-0225, R3-API-METADATA-0226, R3-API-METADATA-0227, R3-API-METADATA-0228, R3-API-METADATA-0229, R3-API-METADATA-0233, R3-API-METADATA-0239, R3-API-METADATA-0251, R3-API-METADATA-0254, R3-API-METADATA-0255, R3-API-METADATA-0256, R3-API-METADATA-0257, R3-API-METADATA-0294, R3-API-METADATA-0304, R3-API-METADATA-0305, R3-API-METADATA-0315, R3-API-METADATA-0319, R3-API-METADATA-0321, R3-API-METADATA-0322, R3-API-METADATA-0323, R3-API-METADATA-0327, R3-API-METADATA-0329, R3-API-METADATA-0331, R3-API-METADATA-0334, R3-API-METADATA-0341, R3-API-METADATA-0354, R3-API-METADATA-0355, R3-API-METADATA-0407, R3-API-METADATA-0408, R3-API-METADATA-0411, R3-API-METADATA-0412, R3-API-METADATA-0433, R3-API-METADATA-0437, R3-API-METADATA-0479, R3-API-METADATA-0494, R3-API-METADATA-0540, R3-API-METADATA-0541, R3-API-METADATA-0548, R3-API-METADATA-0551, R3-API-METADATA-0564, R3-API-METADATA-0565, R3-API-METADATA-0567, R3-API-METADATA-0568, R3-API-METADATA-0571, R3-API-METADATA-0577, R3-API-METADATA-0579, R3-API-METADATA-0592, R3-API-METADATA-0594, R3-API-METADATA-0595, R3-API-METADATA-0596, R3-API-METADATA-0597, R3-API-METADATA-0600, R3-API-METADATA-0601, R3-API-METADATA-0603, R3-API-METADATA-0605, R3-API-METADATA-0606, R3-API-METADATA-0609, R3-API-METADATA-0616, R3-API-METADATA-0618, R3-API-METADATA-0625, R3-API-METADATA-0629, R3-API-METADATA-0632, R3-API-METADATA-0636, R3-API-METADATA-0637, R3-API-METADATA-0638, R3-API-METADATA-0639, R3-API-METADATA-0640, R3-API-METADATA-0643, R3-API-METADATA-0650, R3-API-METADATA-0662, R3-API-METADATA-0666, R3-API-METADATA-0667, R3-API-METADATA-0672, R3-API-METADATA-0675, R3-API-METADATA-0677, R3-API-METADATA-0678, R3-API-METADATA-0684, R3-API-METADATA-0695, R3-API-METADATA-0696, R3-API-METADATA-0697, R3-API-METADATA-0698, R3-API-METADATA-0705, R3-API-METADATA-0706.

All referenced native enum/constant records: CONST-0023, CONST-0033, CONST-0034, CONST-0035, CONST-0036, CONST-0037, CONST-0038, CONST-0045, CONST-0046, CONST-0047, CONST-0048, CONST-0049, CONST-0050, CONST-0051, CONST-0052, CONST-0053, CONST-0054, CONST-0057, CONST-0058, CONST-0060, CONST-0061, CONST-0062, CONST-0063, CONST-0064, CONST-0067, CONST-0069, CONST-0070, CONST-0076, CONST-0078, CONST-0081.

Additional exact constant metadata gates: R3-CONSTANT-METADATA-0035, R3-CONSTANT-METADATA-0036, R3-CONSTANT-METADATA-0037, R3-CONSTANT-METADATA-0038, R3-CONSTANT-METADATA-0047, R3-CONSTANT-METADATA-0048, R3-CONSTANT-METADATA-0049, R3-CONSTANT-METADATA-0050, R3-CONSTANT-METADATA-0051, R3-CONSTANT-METADATA-0052, R3-CONSTANT-METADATA-0053, R3-CONSTANT-METADATA-0054, R3-CONSTANT-METADATA-0057, R3-CONSTANT-METADATA-0062, R3-CONSTANT-METADATA-0063, R3-CONSTANT-METADATA-0064, R3-CONSTANT-METADATA-0069, R3-CONSTANT-METADATA-0070.

### APP_COMPOSITION complete assignments

Original native files: `src/platform/README.md`, `src/platform/macos/README.md`, `src/platform/macos/platform_atomic_file.c`, `src/platform/macos/platform_atomic_new_file.c`, `src/platform/macos/platform_cursor.m`, `src/platform/macos/platform_recovery_store.m`, `src/platform/macos/platform_task.c`, `src/platform/macos/platform_window.m`, `src/platform/macos/platform_window_animation.m`, `src/platform/macos/platform_window_geometry.m`, `src/platform/macos/platform_window_internal.h`, `src/platform/macos/platform_window_native_shell.m`, `src/platform/platform_atomic_file.h`, `src/platform/platform_atomic_new_file.h`, `src/platform/platform_recovery_store.h`, `src/platform/platform_task.h`, `src/platform/platform_window.h`.

All API records: API-0001, API-0032, API-0048, API-0070, API-0071, API-0114, API-0115, API-0116, API-0117, API-0119, API-0122, API-0123, API-0124, API-0126, API-0129, API-0130, API-0133, API-0135, API-0136, API-0142, API-0159, API-0171, API-0173, API-0176, API-0177, API-0185, API-0188, API-0189, API-0191, API-0192, API-0193, API-0201, API-0202, API-0205, API-0208, API-0210, API-0212, API-0216, API-0220, API-0221, API-0222, API-0224, API-0230, API-0238, API-0240, API-0244, API-0245, API-0246, API-0247, API-0258, API-0262, API-0264, API-0266, API-0271, API-0276, API-0280, API-0288, API-0289, API-0290, API-0291, API-0292, API-0296, API-0297, API-0298, API-0302, API-0304, API-0305, API-0307, API-0308, API-0309, API-0310, API-0314, API-0323, API-0324, API-0326, API-0332, API-0333, API-0334, API-0336, API-0337, API-0342, API-0343, API-0351, API-0352, API-0353, API-0358, API-0401, API-0402, API-0403, API-0404, API-0405, API-0410, API-0413, API-0415, API-0418, API-0419, API-0420, API-0422, API-0423, API-0424, API-0425, API-0431, API-0436, API-0438, API-0439, API-0440, API-0441, API-0442, API-0443, API-0444, API-0445, API-0446, API-0455, API-0465, API-0466, API-0473, API-0474, API-0476, API-0483, API-0484, API-0494, API-0509, API-0510, API-0512, API-0513, API-0518, API-0519, API-0520, API-0521, API-0522, API-0523, API-0524, API-0525, API-0529, API-0533, API-0534, API-0538, API-0542, API-0544, API-0545, API-0547, API-0549, API-0552, API-0553, API-0555, API-0556, API-0557, API-0562, API-0570, API-0581, API-0583, API-0584, API-0585, API-0586, API-0596, API-0599, API-0610, API-0612, API-0615, API-0620, API-0624, API-0627, API-0628, API-0634, API-0645, API-0658, API-0664, API-0674, API-0680, API-0686, API-0699, API-0700, API-0701, API-0702, API-0703, API-0704, API-0707, API-0708, API-0709, API-0710, API-0711, API-0712, API-0713, API-0714, API-0718.

Coordinator prerequisites: R3-API-METADATA-0001, R3-API-METADATA-0070, R3-API-METADATA-0071, R3-API-METADATA-0133, R3-API-METADATA-0191, R3-API-METADATA-0193, R3-API-METADATA-0205, R3-API-METADATA-0221, R3-API-METADATA-0296, R3-API-METADATA-0304, R3-API-METADATA-0305, R3-API-METADATA-0323, R3-API-METADATA-0326, R3-API-METADATA-0333, R3-API-METADATA-0334, R3-API-METADATA-0343, R3-API-METADATA-0425, R3-API-METADATA-0439, R3-API-METADATA-0445, R3-API-METADATA-0474, R3-API-METADATA-0494, R3-API-METADATA-0521, R3-API-METADATA-0542, R3-API-METADATA-0547, R3-API-METADATA-0596, R3-API-METADATA-0658, R3-API-METADATA-0674, R3-API-METADATA-0699.

All referenced native enum/constant records: CONST-0027, CONST-0028, CONST-0029, CONST-0032, CONST-0033, CONST-0039, CONST-0040, CONST-0041, CONST-0042, CONST-0043, CONST-0044, CONST-0055, CONST-0056, CONST-0058, CONST-0061, CONST-0066, CONST-0067, CONST-0071, CONST-0072, CONST-0073, CONST-0074, CONST-0077, CONST-0080.

Additional exact constant metadata gates: R3-CONSTANT-METADATA-0032.

## Exact limit and ownership precedence

Every numeric/symbolic anchor in `r3_numeric_and_symbolic_anchors` has original path/function/line/text/hash and exactly one owner. Constants, capacities, test campaign counts, arithmetic overflow checks, enum values and expected bytes remain in the source-bound records; descriptive prose is not an alternative limit. Native Metal and PNG dimensions are **4096**, PNG pixels **16,000,000**, PNG encoded bytes **64MiB**, GIF total frame pixels **32,000,000** (decimal). Menu source and welcome model both use **8** recents; README's 12 is recorded as a documentation mismatch and authorizes no cap change. `platform_settings.m` belongs solely to settings/recents/welcome/localization lane6; lane1 owns `platform_text_ui` preferences/watchers/cache. Composition owns root adapters, not the native lane bodies. Accepted atomic/system/window Rust modules are existing dependencies and remain unchanged.

The R3 preservation audit reads exactly the 83 native, 603 original-source and 157 original-oracle frozen inputs: all match. It does not replace the coordinator's independent acceptance review and does not establish compile, UI, minimum-OS runtime, performance or release readiness.

## Additional exact native constant and shell-style metadata

The inventory and `native-constant-proof.md` add116 referenced native enum/constant records, each with original uses, exact cached constant/enum declaration and feature/Cargo evidence, and local SDK declaration/hash.95 have selected declarations plus public SDK11.1 evidence;21 have individually named coordinator metadata gates. These supplement the718 callable/selector records (504 proved pairs,54 owned callbacks,160 explicitly blocked metadata reviews). No generic class allocation is counted as receiver-class proof without its separate class registration, thread kind and SDK declaration.

The custom welcome/settings/help/feedback text styles include native bold/medium weights beyond the existing widget callback's Regular/Semibold enum. The concrete safe adapter proposal is root-owned `ShellTextRun { text: String, family: NativeUiFamily, weight: f64, size_points: f64, bounds: NativeRect, color: u32 }` and platform-owned `draw_label(&mut self, run: &NativeLabelRequest<'_>, rgba: &mut [u8], width: u32, height: u32, stride: usize) -> Result<(), NativeMediaError>`. NativeLabelRequest carries borrowed UTF8/family, numeric CSS weight100..900, style, finite pixel size, clip and color. Root custom shell models call this shared real CoreText coverage path directly for original500/700-weight runs; ordinary WidgetCanvas labels keep the exact installed TextDraw hook and400/600 weights. Do not silently turn bold into semibold or import NSFont/native controls. Coordinator reviews/registers this owned style metadata before shell implementation; original14 native-shell golden cases remain immutable parity inputs. No current source/interface/dependency is changed.
