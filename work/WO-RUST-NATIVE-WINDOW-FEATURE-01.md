---
id: WO-RUST-NATIVE-WINDOW-FEATURE-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 841597b97d1d53fa680a9f84e2b8ee6450e0e1860617011390b1a4a4405c4c9a
parallel_safe: true
network: false
---
# Exact named scope
Approve/register already-pinned local objc2-app-kit0.3.2 features NSVisualEffectView,
NSImage, NSBezierPath, NSGraphicsContext and objc2-foundation0.3.2 NSProcessInfo
needed by complete owned native window material/custom cursor/clock availability
ports. No version/newpackage/network changes. Record ADR/pinned feature/provenance
supplement, inspect current cached primary manifests/declarations and SDK metadata,
check selected active dependency closure remains existing16 with18lock records,
no dispatch2/io-surface activation. Existing NSVisualEffectView real legacy material
is available baseline; toggle honors preserved macOS26 gate, no new GlassEffectView.
Primitive custom24px diagonal/rotate cursors must port platform_cursor.m, not
replace them with arrow/crosshair. Need no GPL/editor/sourcefetch/new systemtools.
Root only metadata/manifest change, source/nativewindow candidate remains isolated.
Independent scoped Luna review and main lockedoffline build/original7 tests,
protected603/157oracles/HEAD/index/unchangedrustsource audit before acceptance.

# Files allowed to touch
- work/WO-RUST-NATIVE-WINDOW-FEATURE-01.md (coordinator freeze only)
- rust/crates/vexel-platform-apple/Cargo.toml
- Cargo.lock (only local resolution, exact versions preserved)
- docs/adr/0027-native-window-binding-features.md
- docs/dependencies/objc2-native-window.lock.json
- artifacts/WO-RUST-NATIVE-WINDOW-FEATURE-01/**
- STATE.md, LEDGER.md, ledger.jsonl, docs/product/rust-port-goal-and-plan.md

# Protected paths
All product/Rustsource/test/oracles/config/othermanifests/previousorders/evidence/
index/legacy603. Otherroot scene draft source edits authorized only by its own
activeorder and recorded as separatecoordinator delta. No depsdownload/install/
unsupportedOS/newunsafe/Cshim/GUIlaunch/broadUIE2E/stage/commit/reset/delete/agents.
Existing Standard subscription only, no APIbilling or extra credits.

# Verification
cargo +stable generate-lockfile --offline
cargo +stable check -p vexel-platform-apple --locked --offline
cargo +stable test -p vexel-platform-apple --locked --offline
cargo +stable tree -p vexel-platform-apple --locked --offline --edges normal,build --prefix none --format '{p}'
python3 scripts/wo_validate.py WO-RUST-NATIVE-WINDOW-FEATURE-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Lockedexternal pins/checksums unchanged, selectedgraph/no optional activation;
source/index audit and independentreview. No native functionality/runtime claim.

# Output
Complete metadata-only selected-feature prerequisite for full nativewindow repair;
source candidate not accepted, originalR1 scope maintained/preserved.
