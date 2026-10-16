---
id: WO-RUST-PROJECT-IO-LOAD-01
revision: 1
status: READY
milestone: V1-PROJECT-OPEN
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 39e774f72b957aa89426c937d7089a17866e280b78885c70691e8d102bb89fab
parallel_safe: true
network: false
---
# Goal
Add a bounded, validating native `.vexel` file-load API to the accepted
`vexel-project-io` Rust crate. Return an owned immutable encoded bundle whose
validated `BundleView` borrows from that owned storage, without a self-referential
struct or exposing partial bytes on failure. This is a project I/O primitive for
the planned V1 app workflow; it does not add app Open UI or complete V1.

# Required behavior
1. Expose a small safe API taking caller-owned macOS path bytes. Validate empty,
   NUL-containing, and over-limit paths before file access, using the already
   accepted `MAX_ATOMIC_FILE_PATH_BYTES` boundary. Preserve the established
   `FileError` / `vexel_project::Error` categories where applicable.
2. Open and inspect the selected file; reject non-regular files and any declared
   size above `vexel_project::MAX_BYTES` before allocating a payload buffer.
   Metadata is only an early check: enforce the same hard limit while reading in
   bounded chunks in case a file grows after inspection. Retain no more than
   `MAX_BYTES`; use checked arithmetic and fallible `Vec::try_reserve` before
   extending. Never use unbounded `read_to_end`, `read_exact` into a claimed
   size, or trust metadata as proof of final size.
3. Decode and validate the complete bundle before returning success. On bad
   magic, unsupported version, corrupt/checksum-invalid data, limit, allocation,
   or I/O error, return no loaded project and no partial bytes. Preserve the
   bundle decoder's typed error and offset through the public result/view API.
4. The successful loaded value owns the exact encoded bytes and exposes a
   lifetime-safe method for callers to obtain `BundleView<'_>` and access SVG,
   timeline, manifest, and assets. Do not retain an invalid view, raw pointer, or
   duplicate copy of the encoded bundle. Document ownership and the up-to-1-GiB
   format cap honestly; do not claim streaming decode or a low-memory promise.
5. Add focused tests using test-owned paths under `.cache/project-load-tests`
   and private small-limit reader tests. Cover valid file load and exact payload
   preservation, invalid path before file access, missing file/directory,
   oversized sparse-file rejection without reading/allocating the payload,
   bad magic/version/checksum error propagation, short reads, a stream growing
   past the limit, injected read failure, and absence of a partial loaded value.
   Do not allocate a 1-GiB fixture; describe small-limit tests as boundary math
   and the sparse file as a metadata precheck.
6. Keep new source and test files below 800 lines. Safe Rust only; no new
   dependencies, production changes to the bundle codec, OS portability code,
   app UI, or modification of prior assertions/oracles.

# Files allowed to touch
- `rust/crates/vexel-project-io/src/file_load.rs` (new)
- `rust/crates/vexel-project-io/src/load_tests.rs` (new)
- `rust/crates/vexel-project-io/src/lib.rs` (module and public exports only)
- `artifacts/WO-RUST-PROJECT-IO-LOAD-01/**` except immutable source seed

# Protected
All other source, tests, fixtures/oracles, manifests/locks, work orders, docs,
shared state/ledgers, HEAD/index, and user files. No new dependency, network,
GUI, broad UI/E2E, app launch, performance claim, staging, commit, reset, or
delete. Candidate work must be isolated; coordinator alone integrates.

# Verification
- `cargo +stable check -p vexel-project-io --locked --offline`
- `cargo +stable test -p vexel-project-io --locked --offline`
- `cargo +stable fmt --package vexel-project-io -- --check`
- `python3 scripts/wo_validate.py WO-RUST-PROJECT-IO-LOAD-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- Exact source-seed/scope/hash/line-count audit and `git diff --check`.

Independent review and fresh current-main checks are required before
integration. Keep broad UI/E2E deferred. This order establishes validated file
loading only; app document replacement, native Open/Save panels, drawing,
rendering, animation, MCP app control, and full-product acceptance remain open.
