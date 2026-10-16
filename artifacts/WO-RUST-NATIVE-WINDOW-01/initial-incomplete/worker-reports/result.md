# Work order result

**Status: incomplete candidate; not ready to integrate.** This change adds the
public Rust-owned value/API surface, a real AppKit `NSApplication`/`NSWindow`
creation and basic lifecycle path, generation-checked weak window handles,
bounded event queue primitives, text snapshot validation, and pure geometry,
modifier, UTF-16, and queue tests. The seven original native-boundary checks
remain untouched and pass.

The full accepted order is not complete. There is no custom responder/window
delegate, inbound callback route registry, native event dispatch, complete
`NSTextInputClient`, close/focus/appearance/resize callback event path, actual
removable per-window timer or run-loop wakeup, complete Quartz display-origin
conversion, or callback-fault test harness. The public tick request currently
only records a flag. Several cursor variants and material handling explicitly
return an error instead of pretending to provide the missing native behavior.

Specific selected-binding blocker: the immutable `vexel-platform-apple`
manifest enables `NSTextInputClient` but omits the `NSVisualEffectView` feature
and type needed for the legacy toolbar material view. This order forbids
manifest/feature changes, so `set_material_enabled` cannot implement that
contract with the selected API. No substitute visual effect was added.

Do not accept or integrate this candidate as the completed window lane. No GUI
was launched; no macOS 13 runtime, callback, native IME, presentation, or
performance claim is made. See `parity-map.json` for per-source status and
`verification.txt` for exact offline evidence.
