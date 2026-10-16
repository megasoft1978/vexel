# ADR0027: native window binding features

Status: Accepted metadata prerequisite; independent Luna review and locked offline checks recorded in artifacts/WO-RUST-NATIVE-WINDOW-FEATURE-01.
Date: 2026-10-14. Work order: WO-RUST-NATIVE-WINDOW-FEATURE-01.

The first native-window candidate lacks the selected NSVisualEffectView binding
needed by the preserved real window container. Activate this and NSImage,
NSBezierPath, NSGraphicsContext for the existing owned24px custom diagonal/rotate
cursors in platform_cursor.m, and Foundation NSProcessInfo for monotonic uptime
and checked OS availability. All use the already-audited exact0.3.2 packages;
no new package/version/download or first-party foreign shim is introduced.

NSVisualEffectView is the actual preserved class, not NSGlassEffectView. Existing
source creates a titlebar material view and only enables it at macOS26+. Preserve
that gate on mac13+; older systems keep the material hidden without invoking a
newer runtime-only class. Custom cursor geometry/halo/hotspot semantics remain
owned code. Normal baseline cursors continue through NSCursor.

Cached primary declarations and manifests provide exact feature closure and ABI
provenance; the installed SDK and prior accepted ADR0026/licensing/native boundary
remain authoritative. This supplements its selected features only. All native
objects stay private/main-thread-owned, outbound and inbound guards preserved;
no new foreign-unwind assumption, manual objc_msgSend/transmute or unchecked
marker. Release attribution and real mac13 GUI/IME/cursor/material testing remain
final gates. Compilation proves selected API availability, not runtime fidelity.
