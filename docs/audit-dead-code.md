# Rust dead-code and dependency audit

Scope: `rust/` workspace source and manifests, excluding the prohibited ledger,
state, and artifact paths. This is a static audit; no source or manifest changes
were made. The workspace currently has no confirmed removable crate or
dependency based on the checks below.

## Findings

### Document crate chain is used transitively

- **Evidence:** `cargo tree --workspace --locked -i vexel-document` shows
  `vexel-document-edit -> vexel-document`,
  `vexel-v1-document -> vexel-document-edit`, and
  `vexel-app -> vexel-v1-document`. The inverse trees for
  `vexel-document-io`, `vexel-document-edit`, and `vexel-v1-document` show the
  same active path to `vexel-app` (and `vexel-corpus` through the app).
- **Disposition:** **Needs decision** only if the product architecture changes;
  not safe to remove in this audit. Lack of a direct `vexel-app` dependency on
  the first two crates does not make them unused.
- **Verify:** `cargo tree --workspace --locked -i vexel-document` (repeat with
  `vexel-document-io`, `vexel-document-edit`, and `vexel-v1-document`).

### No unused direct dependencies confirmed in `vexel-app`

- **Evidence:** Every path dependency in
  `rust/crates/vexel-app/Cargo.toml` has at least one identifier reference in
  `rust/crates/vexel-app/src/`. The source scan includes app tests under `src/`
  as well as production modules. `cargo-machete` and `cargo-udeps` are not
  installed, so no dedicated unused-dependency analyzer was available.
- **Disposition:** No safe removal identified. A textual reference is a useful
  screen, not proof that every dependency is required; verify with a dedicated
  analyzer before pruning any manifest entry.
- **Verify:** Compare path dependencies in
  `rust/crates/vexel-app/Cargo.toml` with `rg -n '<crate_identifier>'
  rust/crates/vexel-app/src`; if installed, run `cargo machete --workspace`.

### `#[allow(dead_code)]` annotations exist outside `vexel-app`

- **Evidence:** `rg -n '#\[allow\(dead_code\)\]' rust --glob '*.rs'` finds
  annotations in `vexel-ui/src/icons_draw.rs`,
  `vexel-platform-apple/src/native_window/window.rs`,
  `vexel-platform-apple/src/main_thread.rs`, and
  `vexel-svg-reference/src/lib.rs`, plus test-only annotations in SVG
  definitions and CSS tests. The `main_thread.rs` annotation states that a
  method is used by follow-on native modules. No such annotation was found in
  `vexel-app`.
- **Disposition:** **Needs decision** per annotation. Some are documented
  forward-use or feature/test support; deleting them or their targets without
  checking conditional compilation and planned integration risks breaking
  intended work.
- **Verify:** `rg -n -C 3 '#\[allow\(dead_code\)\]' rust --glob '*.rs'` and
  inspect each surrounding cfg/module and call site.

### `vexel-app` public API cannot be classified as unused from in-workspace calls

- **Evidence:** `rust/crates/vexel-app/src/lib.rs` exposes many public modules.
  Public items may be consumed by external users, and `vexel-corpus` consumes
  the `vexel-app` crate. A repository-wide search for a public item with no
  in-repo call site cannot establish that the API is dead.
- **Disposition:** **Needs decision** before narrowing visibility or removing
  public items. No specific unreferenced item is marked safe to remove by this
  static pass.
- **Verify:** `rg -n '\b<ItemName>\b' rust --glob '*.rs'` for a candidate,
  followed by checking downstream consumers and intended public API.

## Verification run

- `cargo tree --workspace --locked -e normal` — completed; workspace graph
  includes the candidate document crates.
- `cargo check --workspace --locked` — passed.
- `cargo machete` / `cargo udeps` — unavailable; neither executable is installed.
- No focused test target was specified for this read-only audit; tests were not
  run.
