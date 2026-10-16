# App lane candidate evidence

Candidate worktree: `.cache/WO-RUST-MCP-LIVE-PREVIEW-01/app`  
Base: `19234a036ed6a8d40c2ccdaec9b8186806d70eca`

## App changes

- Added `--mcp-preview` native-window launch while leaving default launch on
  the existing shell path.
- Added a bounded stdio server thread and bounded synchronous UI request queue.
  The UI thread owns the preview document and commits staged edits only after
  `present_rgba` succeeds; the backend receives its tool response afterward.
- Added a 512×512 source-owning V1 document, bounded rect/ellipse mapping,
  source-rebuilding undo/redo history, a 128-shape limit, exact attribute and
  color validation, bounded primitive rasterization, and MCP activity text.
- Added focused tests for argument mapping, invalid-input atomicity, source and
  history restoration, shape raster bounds/pixels, scene capacity, and
  request-to-presentation response ordering.

## Verification

Cargo check and tests ran in the isolated generated workspace at
`.cache/build/app-check`, using its generated lock and candidate-local target
directory. This avoids updating the frozen `Cargo.lock` in the app candidate.

```text
CARGO_HOME=/Users/megasoft78/Desktop/Freelance/vexel/.cache/cargo
CARGO_TARGET_DIR=/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-MCP-LIVE-PREVIEW-01/app/.cache/build/app
cargo +stable check -p vexel-app --offline
Finished `dev` profile

cargo +stable test -p vexel-app --offline
16 passed; 0 failed

cargo +stable fmt -p vexel-app -- --check
passed

python3 scripts/wo_validate.py WO-RUST-MCP-LIVE-PREVIEW-01 --ready
valid revision 1; SHA-256 20ff1c0d0f8e2144c012a54396fc608a285316b0214bf1a2f070644db29d1c06
```

The 87-file source-seed audit found changes only in the three app files that
were present in the seed and the coordinator-synced protocol files. The frozen
candidate `Cargo.lock` matches SHA-256
`4e52f7e8c42f8e56820ba21c9be46c0b6727d259eece19a0c7ade6d36b4b8592`.

## App source hashes

```text
de4ebd59ff9cc4efd947eb2caa8eb812170813734ee9a3ad04856e68c28bbe1d  rust/crates/vexel-app/Cargo.toml
a3ed951226e47b4c9b1901c8c670dcff2e7d22092a9c8d1d45dd87e322bd503b  rust/crates/vexel-app/README.md
ca9b299b30cc764a9e94ab30a6778fa4b109e90418784c5e57de8d6d7b1ff99d  rust/crates/vexel-app/src/main.rs
ab28f4941fd9b57685d28a72d8ef5fbe2cb2ab0306e1ce303ee2296258860762  rust/crates/vexel-app/src/preview.rs
f0f2582d213335dca53ae343d337385d683bbef98513975e055ef2bca0526cd9  rust/crates/vexel-app/src/preview_app.rs
```

Native GUI rendering and an actual Codex MCP session were not run; the work
order defers visible GUI automation, and no isolated display was configured.
