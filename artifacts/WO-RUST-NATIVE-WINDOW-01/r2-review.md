**BLOCKER — queue capacity does not enforce the frozen 240 ordinary / 16 reserved split.**

In [queue.rs](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-NATIVE-WINDOW-01/worker/rust/crates/vexel-platform-apple/src/native_window/queue.rs:124), ordinary occupancy excludes `is_control` events, but insertion only checks the overall 256-event cap. Repeated alternating `Focus` and `AppearanceChanged` events therefore bypass the ordinary count and can occupy all 256 entries; the 17th control event is accepted instead of exhausting the reserved slots and sequencing fault/reset recovery.

Enforce the 16-slot reserved limit while preserving sticky fault/reset/close delivery, and add a focused saturation proof for control-event pressure.

I reviewed the R2 work order, owner directives, source bridge, native-window modules, tests, and candidate artifacts. The artifacts report the requested 40 focused checks and preserve the stated runtime limitations; I did not rerun them under the read-only instruction. The native AppKit, IME, geometry, ABI, and teardown paths appear represented in the source candidate, but this queue-contract defect blocks source-only acceptance.