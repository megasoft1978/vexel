# WO-RUST-V1-RELEASE-CI-01 result (candidate)

- Clippy: available on this host (0.1.99) but fails on pre-existing deny-level errors in vexel-svg-decode (clippy.log). CI step is conditional on `cargo clippy --version` and `continue-on-error` (informational; no -D warnings gate).
- vexel-app tests added to CI (`cargo test --workspace --locked`): they pass headless on this host. Full frozen run: 330 passed, 0 failed, 4 ignored (reproducers).
- Unexecuted: Developer ID signing, notarization, live GUI launch (coordinator check). Ad-hoc codesign only.
- Placeholder bundle id `app.vexel.Vexel`; no icon supplied (no CFBundleIconFile; `--icon FILE.icns` supported).
- fmt: 16 crates clean-listed (incl. vexel-dev, formatted), 27 in fmt-debt (28 drifted before formatting vexel-dev, not 29).
- Baselines (debug, Apple M5 Max): xml tokenize 1 MiB 6.41 ms (max 250); compose 2000 rects 512x512 3178.29 ms (max 25500); 1000 tools/list 85.38 ms (max 700; spread over 10 sessions because the protocol rate-limits 120 req/min per session).
- Bundle: build/release/Vexel.app (2.5 MB, exe 2.4 MB), Vexel.dmg 1.4 MB, sbom.cyclonedx.json (11 components); not committed. hdiutil info identical before/after smoke.
- make_sbom.py: version read from Cargo.toml, default binary/out moved to build/release, error text.
- vexel-dev honours CARGO_HOME/CARGO_TARGET_DIR env (else repo-local).
- Gaps: workflow_dispatch smoke unverified on GitHub; vexel-dev needs `rustc +stable` == 1.99.0 on the runner. git refused for the worker; nothing committed.
