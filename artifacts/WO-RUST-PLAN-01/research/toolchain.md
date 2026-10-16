## Findings (research checked 2026-10-14)

### Decisions

- **Toolchain:** pin Rust **1.99.0** as the initial stable toolchain, with Edition **2024** and resolver **3**. Rust 1.99.0 was released October 1, 2026; Cargo documents Edition 2024 and virtual workspace resolver configuration. This is a research recommendation, not a claim that the installed toolchain was checked. ([Rust 1.99.0 release](https://blog.rust-lang.org/2026/10/01/Rust-1.99.0/), [Cargo workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html))

- **Platform target:** build only `aarch64-apple-darwin`, set `MACOSX_DEPLOYMENT_TARGET=13.0`, and keep Cocoa/Metal calls behind Rust `platform/macos` interfaces. Rust supports that target from macOS 11 onward and recognizes `MACOSX_DEPLOYMENT_TARGET`; the repository’s stricter macOS 13 minimum remains controlling. ([Rust Apple targets](https://doc.rust-lang.org/rustc/platform-support/apple-darwin.html), [Apple build settings](https://developer.apple.com/documentation/xcode/build-settings-reference))

- **Workspace/dependencies:** start with no third-party crates. Use a virtual root `Cargo.toml`, shared `Cargo.lock`, and `workspace.package`/`workspace.dependencies` ownership centralized with the coordinator. Suggested initial layout:
  ```
  app/vexel/
  src/{base,platform/macos,xml,svg,scene,raster,document,animation,ui}/
  tools/dev/
  ```
  Keep native APIs isolated; do not create unsupported-platform stubs. If Rust tooling needs JSON parsing, evaluate **serde_json = "=1.0.146"** as a narrowly scoped exception: the official tagged manifest says MIT OR Apache-2.0 and Rust 1.68 minimum; it brings serde_core, itoa, memchr, and ryu runtime dependencies. Record the tag/revision, license, transitive graph and hashes in a reviewed provenance record before adoption. ([serde_json 1.0.146 manifest](https://github.com/serde-rs/json/blob/v1.0.146/Cargo.toml); tag checked October 14, 2026.)

### Scope and ownership

CMake currently composes the app, multiple library/test/CLI targets, Cocoa/Metal frameworks, and a Python-driven `.app` bundle step. `dev` dispatches build, test, rendering, fuzz, benchmark, smoke, packaging and release workflows. Python scripts cover ledger/work-order runners, tests, corpus handling, render comparisons, perf gates, fuzzing, UI automation and packaging.

Cover these responsibilities in the eventual Rust tooling migration: `CMakeLists.txt`, `dev`, `scripts/**`, `packaging/**`, `app/test_driver/**`, and Python/shell test harnesses under `tests/**`. Preserve fixtures, schemas, artifacts and legacy source as references; port behavior and contracts without broadening platform support. Split tooling into `tools/dev` (command dispatch/build/test orchestration), `tools/ledger`, `tools/work-order`, `tools/bench`, and `tools/packaging` only as ownership boundaries become clear. Keep app and tooling crate dependencies explicit and minimal.

### Build and release tradeoffs

Use Cargo profiles as the replacement for Debug/Release configuration, with generated output in repository-local `target/`. Rust’s documented Release defaults include `opt-level=3`, no debug info, 16 codegen units, and no LTO; fewer codegen units can trade longer compile time for optimization opportunities. Benchmark candidate settings on the actual app before selecting size/performance policies. `opt-level="s"`/`"z"`, LTO, stripping and panic strategy are candidates to measure, not guaranteed binary-size wins. Do not claim Rust will be faster or smaller without measurements. ([Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html), [rustc codegen options](https://doc.rust-lang.org/rustc/codegen-options/index.html))

For controlled dependency builds, require committed lockfile and `--locked`; use offline/frozen verification only after reviewed dependency fetches. ([Cargo vendor and locked/offline behavior](https://doc.rust-lang.org/cargo/commands/cargo-vendor.html))

### Ready work-order sequence

1. **Rust bootstrap:** coordinator owns root manifest, lockfile, toolchain pin, Cargo config, initial `app/vexel` and `tools/dev` shells, macOS 13 target enforcement, and a no-network build/release wrapper replacing only the simplest `dev build` path. No C removal or parity claim.
2. **Build/package parity:** port app/test target graph, bundle metadata/resources/icon generation, and Debug/Release output layout.
3. **Tooling foundations:** Rust ledger/schema validation, work-order validation and runners, then Python unit-test harness contracts.
4. **Specialist tools:** corpus, golden image comparison, fuzz orchestration, benchmarks/perf gates, UI driver and packaging; separate orders with frozen interfaces and preserved evidence.
5. **Retire CMake/Python entry points** only after Rust parity gates cover each command and the owner accepts results.

### Risks and unknowns

The root Cargo manifest, crate boundaries, macOS framework bindings strategy, signing/notarization flow, and Rust equivalents for Python imaging/XML tools require specific follow-up research and ownership decisions. Dependency selection must account for MPL-2.0 project policy, transitive licenses, hashes and supply-chain review. Cargo’s deployment-target environment behavior still needs a real macOS 13 compatibility build; no build was run for this read-only task.