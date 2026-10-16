# ADR-0030: enable objc2-foundation NSURL for native open/save panels

Status: Accepted (coordinator amendment to ADR-0026), 2026-10-16. Work order: WO-RUST-V1-NATIVE-PANELS-01.

## Decision

Add the `NSURL` feature of the already-pinned `objc2-foundation =0.3.2` to the
locked feature set (`docs/dependencies/objc2-macos.lock.json`) and to
`vexel-platform-apple`. `NSOpenPanel`/`NSSavePanel` results are `NSURL`. No new
package, version or license enters the dependency closure; `Cargo.lock` is unchanged.
All other ADR-0026 constraints (main-thread capability, no Objective-C exception or
panic crossing a boundary, first-party Rust only) still apply.
