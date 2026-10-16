# Rust native bindings audit — blocked checkpoint

2026-10-14 · WO-RUST-NATIVE-BINDINGS-AUDIT-01 · macOS 13+ / Apple Silicon

The dependency prerequisite remains **unresolved**. The first required primary
registry request, `https://crates.io/api/v1/crates/objc2`, failed with
`<urlopen error [Errno 8] nodename nor servname provided, or not known>`.
The bounded stdlib HTTPS reader used a 25-second timeout and 4,000,000-byte
metadata ceiling. It received zero bytes. Network work stopped after this one
attempt; no retries, alternate hosts, archives or Cargo requests followed.
This is a checkpoint, not an adoption-ready audit or completed work order.

## Authority and evidence

Read OWNER-DIRECTIVES, AGENTS, the ready order, ADR0022–0025 and the accepted
`artifacts/WO-RUST-NATIVE-PLAN-01/result.md`. Inspected original window/input,
native shell, geometry, timer, menus, AX, dialogs, clipboard, text and Metal
contracts and relevant implementation excerpts. Their hashes are recorded in
`artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01/local-source-provenance.json`.
Existing legacy behavior is migration evidence, not authority to omit the
proposal's IME, ownership, queue, per-window GPU or teardown requirements.

The installed SDK queries returned version **27.0**, at
`/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs/MacOSX27.0.sdk`.
Xcode emitted filesystem/cache warnings; both queries succeeded. SDKSettings
and relevant header hashes, bounded declaration excerpts and line references
are retained under the audit directory. No published binding generator SDK
provenance was obtained. Current SDK declarations alone do not prove macOS 13
runtime compatibility.

## Pins, features, closure and licenses

No exact pin is recommended. Requested framework **0.3.2** is a candidate to
reconcile against publication metadata, not a verified available release.
Current objc2, objc2-encode and block2 versions are unknown here. Foundation,
AppKit, CoreFoundation, CoreGraphics, CoreText, Metal and QuartzCore metadata
were not requested after the first failure. Published checksums, normalized
manifests, original manifests, feature names, license expressions/texts, MSRVs
and transitive dependencies therefore remain unverified. No license
compatibility or Rust 1.99 compilation claim is justified by this checkpoint.

`pins-features-licenses.json` deliberately records null pins, features,
checksums, licenses and closure. `default_features=false` records the requested
future policy; it is not a resolved configuration. Do not substitute SDK header
names for Cargo feature names or assume objc2-encode/block2 are the entire
closure. Exception support may activate additional helper/build dependencies;
that must be established from published manifests and implementation.

Future resolution must verify publication/yank status and checksums, read
licensed bounded archives, and expand features to a fixed point. Include
optional `dep:` activation, dependency default features, strong/weak dependency
feature edges, target predicates for Apple Silicon macOS and build dependencies.
Constrain every activated version by published semver ranges and resolver 3's
Rust compatibility rules. Record a complete pinned package/feature/edge/license
graph. ADR0023's Rust 1.99/2024 toolchain requirement is locally established;
binding compatibility still requires primary MSRV evidence and later authorized
type checking. Dependency adoption requires its separate ADR/order.

## Capability matrix

The following is an Apple-symbol investigation map. **Every Rust export and
minimal Cargo feature mapping is unresolved.** `symbols.json` supplies 34
header groups with exact local line references and candidate framework crates.
It labels partial availability checks explicitly; it is not a verified
symbol-feature matrix.

| Scope | Candidate Apple surface | Binding candidate / remaining condition |
|---|---|---|
| Lifecycle, dispatch, wake, shutdown | NSApplication sharedApplication, activation policy, finishLaunching, nextEventMatchingMask, sendEvent, stop; NSRunLoop | AppKit/Foundation; bounded turns, autorelease pools and wake semantics need binding audit |
| Windows, delegates, close/fullscreen | NSWindow initWithContentRect, delegate callbacks, close; NSView | AppKit; fallible creation, generation routing, retained ownership, detached delegates |
| Key/modifier/pointer/scroll/magnify | NSResponder keyDown/keyUp/flagsChanged/scrollWheel/magnifyWithEvent; NSEvent | AppKit; explicit modifier translation, finite coordinates, bounded owned queue/coalescing/fault policy |
| IME, committed/marked text | interpretKeyEvents; NSTextInputClient insertText/setMarkedText/unmarkText, ranges, candidate rectangle, character index | AppKit/Foundation; implement complete protocol, UTF-16 range conversion and composition; legacy charactersIgnoringModifiers is insufficient |
| Geometry, backing, cursors | NSView conversion, NSWindow/NSScreen; CGDisplayBounds; NSCursor | AppKit/CoreGraphics; per-display conversion and accepted scale validation |
| Drops, startup/reopen/recent files | NSDragging, NSPasteboard file URLs; NSApplicationDelegate openFiles/openURLs/reopen; NSURL | AppKit/Foundation; bounded owned paths and command routing |
| Menus, shortcuts, dock, state | NSMenu/NSMenuItem, delegates, target/action | AppKit/Foundation; snapshots and one typed command per action |
| Accessibility trees/actions | NSAccessibilityElement/protocol/constants, frame conversion, notification functions | AppKit; validated owned semantics and generation-tagged actions |
| Dialogs, alerts, unsaved/recovery/export | NSOpenPanel/NSSavePanel, NSAlert, sheets | AppKit/Foundation; cancellation, late callbacks, path bounds; content-type feature dependencies unknown |
| Clipboard | NSPasteboard setString/setData/readObjects/dataForType | AppKit/Foundation; SVG/text/PNG limits and owned copies |
| Preferences/appearance | NSWorkspace accessibility flags, NSAppearance/NSColor, NSUserDefaults, notification/KVO observers | AppKit/Foundation; removable subscriptions, appearance invalidation, exception policy |
| Animation ticks | NSTimer, invalidate, run loop | Foundation; removable timer, stale generation rejection |
| Font shaping/outlines | CTFont/CTLine/CTRun, CF containers/attributed strings; CGPathApply | CoreText/CoreFoundation/CoreGraphics/Foundation; 64-KiB/16,384-glyph/262,144-segment limits, unsupported color glyphs |
| Custom labels | CGBitmapContextCreate, CGPath and CGContext operations | CoreGraphics/CoreText; checked borrowed RGBA stride/clipping, owned layout/cache; no retained pixel borrow |
| GPU presentation/readback | MTLDevice/queue/texture/library/pipeline/command buffer/encoders; CAMetalLayer/drawable | Metal/QuartzCore; per-window resources, bounded in-flight retention, copy before return, recoverable drawable absence |

No additional UI toolkit, dispatch crate or C shim is authorized. Welcome/editor
widgets remain custom Rust UI; native bindings supply these platform services.

## Thread, callback, block and exception boundary

The local NSThread header declares `isMainThread`; the published
`MainThreadMarker` API and auto-trait guarantees could not be inspected.
Preserve proposal main-thread capabilities and `!Send + !Sync` native owners.
Callbacks enqueue owned data without borrowing the model or reentrant mutation.
Invalidate routing before unregistering observers/timers, canceling sheets,
detaching delegates and releasing objects. GPU completion may arrive after
teardown; retain resources and discard stale generations.

Apple headers establish concrete block demand: MTLCommandBuffer
`addCompletedHandler:`, NSSavePanel/NSAlert sheets and NSNotificationCenter
`addObserverForName:object:queue:usingBlock:`. This justifies investigating block2,
not adopting it. Its heap-block copying, capture lifetimes, thread requirements,
Rust exports and feature/license closure remain unknown. CGPathApply supplies a
C function/context alternative for outline enumeration; block use there is
unnecessary. Legacy dispatch semaphore usage does not mandate dispatch bindings;
a future Rust completion state still needs a reviewed lifetime/thread contract.

No Rust panic may cross Objective-C, C or block callbacks. Catch-unwind policy
requires later verification of panic configuration; abort behavior is separate.
It does not catch Objective-C exceptions. Legacy preference KVO registration and
removal explicitly use `@try/@catch`. Published objc2 exception APIs, foreign
unwind guarantees and any helper code/build dependencies are unverified. An
all-Rust boundary without a first-party C shim must resolve this before adoption.

## Availability and remaining gates

Exact local annotations establish notification blocks at macOS 10.6, AX
notification-with-user-info at 10.7, contrast/transparency flags and accessibility
display notification at 10.10, reduced motion at 10.12, and CGPathApplyWithBlock
at 10.13. These declarations are compatible with the stated minimum, without
constituting runtime evidence. Other matrix entries require complete enclosing
class/protocol/constant and method availability review.

NSGlassEffectView is explicitly macOS **26.0**; its `effectIsInteractive`
property is **27.0**. Exclude these from the baseline or guard them with an
approved macOS-13 custom opaque alternative. Do not enable a broad framework
feature set that silently admits post-13 symbols. Published generated bindings
must be cross-checked against every selected SDK declaration, including
protocol requirements, weak linking and content-type dependencies.

Remaining gates: restore primary-source access in a separately authorized run;
complete exact pins/feature closure/licenses/provenance/MSRV and boundary API
inspection; finish symbol availability review; obtain coordinator independent
focused review. Later adoption must verify compilation/linking and macOS-13
runtime behavior, IME, VoiceOver, observer teardown and GPU completion safety.
No compilation, runtime or GUI acceptance was attempted here.

## Verification

`python3 scripts/wo_validate.py WO-RUST-NATIVE-BINDINGS-AUDIT-01 --ready`
passed revision 1/spec hash
`c6c015d1b3ab478729475fb46d468d97c2cbebe6a0e9ab5e7880589de4103da6`.
`python3 scripts/ledger.py validate ledger.jsonl` passed 1,292 entries.
`git diff --check` passed. Exact outputs are in `verification.json`.

All **603** original baseline files and **130** audit Rust baseline files match
their recorded hashes; HEAD and index hashes match both baselines. See
`preservation-legacy.json` and `preservation-rust.json`. Git's whitespace check
does not cover untracked outputs; local evidence validation covers the new
report/artifacts separately. No product, peer, manifest, lock, toolchain, ledger,
index or HEAD edit was made; no tests, installs, Cargo downloads, paid API,
recursive agents or GPL editor source were used.
