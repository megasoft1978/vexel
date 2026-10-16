**ACCEPT** the proposed [ADR0026](/Users/megasoft78/Desktop/Freelance/vexel/docs/adr/0026-rust-apple-bindings-and-abi.md) and [provenance lockfile](/Users/megasoft78/Desktop/Freelance/vexel/docs/dependencies/objc2-macos.lock.json). No blocking technical, owner-policy, or provenance issue found.

The 16 package records exactly match the accepted audit index. Root selections differ only by Foundation `NSException`; its published feature adds no dependency. That extension is justified for real exception probes and diagnostics. Defaults remain disabled at the roots, with unavoidable transitive defaults and host build utilities documented by the audit.

The ABI policy is enforceable within its stated limits. Published message-send code catches Objective-C exceptions through the native helper, then raises a Rust panic. The helper explicitly uses `C-unwind`, catches Objective-C objects, and lets Rust unwinds continue. This supports guards at public operations and inside inbound callbacks without claiming zero foreign unwinding through Rust frames. Primary Rust ABI documentation supports the distinction: unwind-capable declarations permit passage; `catch_unwind` does not establish reliable foreign-exception recovery.

Implementation review must verify that fault conversion, payload disposal, callback cleanup, and fallback return stay within the guard or documented fatal-case handling. Those obligations are already present in the frozen boundary order. Raw C/CF, retain/release/Drop, exceptional cleanup, allocator failure, and abort limitations are explicit.

The external Objective-C helper is a reviewed dependency exception; first-party implementations remain Rust behind the private platform boundary. Main-thread capability, opaque pointers, and reentrancy restrictions preserve owner requirements.

SDK27 evidence and current-host checks remain distinct from macOS13 runtime acceptance. License selection preserves published grants and notices without inventing attribution, claiming legal clearance, or accepting agreements; release attribution and distribution review remain later gates.

This accepts the proposed decision and provenance only. Adoption/fetch, Cargo resolution, compilation, and real boundary probes remain downstream. No network, installs, builds, tests, agents, or file changes occurred.