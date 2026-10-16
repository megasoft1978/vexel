---
id: WO-RUST-RASTER-CAPACITY-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-coordinator
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: d74179011919a27517b51c09300ac7a89f54b3c262546727cb0bb12d3015433a
parallel_safe: false
network: false
---
# Goal
Resolve tile-cache reserved-payload accounting dependency with a safe accepted Tile capacity accessor and useful owned-RGBA import contract; preserve all existing Tile methods/error enum/5assertion cases.

# Files allowed to touch
- `work/WO-RUST-RASTER-CAPACITY-01.md`
- `rust/crates/vexel-raster/src/lib.rs`, `rust/crates/vexel-raster/tests/owned_rgba.rs`
- `artifacts/WO-RUST-RASTER-CAPACITY-01/**`, `artifacts/WO-RUST-TILE-CACHE-01/**`
- `.worktrees/rust3-cache/rust/crates/vexel-raster/src/lib.rs`, `.worktrees/rust3-cache/rust/crates/vexel-raster/tests/owned_rgba.rs`
- `.worktrees/rust3-cache/work/WO-RUST-RASTER-CAPACITY-01.md`, `.worktrees/rust3-cache/ledger.jsonl`
- `.worktrees/rust3-cache/artifacts/WO-RUST-TILE-CACHE-01/**`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md`

# Protected paths and contract
All manifests/lock, other crate source, accepted existing Tile behavior/test bodies, legacy sources/fixtures/oracles, other workers and frozen orders/evidence. No deps/install/unsafe/network/UIcampaign. New APIs Tile::allocated_byte_count()->usize reports actual Vec capacity, Tile::from_owned_rgba(width,height,Vec<u8>)->Result<Tile,(TileImportError,Vec<u8>)> retains ownership/bytes/capacity on failure, imports without clone/resize/allocation and validates dimensions/exactpixel-length/allocated-cap<=64MiB. Separate TileImportError preserves existing TileError enum; MAX_ALLOCATED_BYTES=64MiB matches4096axisRGBA. No unsafe/forged capacity; test realistic spare-capacity buffer, invalid layout/length/cap retention. Cache must charge allocated capacity, not pixel len; actual allocator hidden bookkeeping/RSS is not claimed. Coordinate approved dependency delta into cache overlay/local ledger without changing other workers or dropping baseline history. Small typed interface extension, no renderer/app completion claim.

# Verification
cargo +stable check -p vexel-raster --locked --offline
cargo +stable test -p vexel-raster --locked --offline
cargo +stable check -p vexel-ui --locked --offline
python3 scripts/wo_validate.py WO-RUST-RASTER-CAPACITY-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Exactexisting1.99/macOS13 and local outputs. Root review ownership/overflow/failure tests and original source/index preservation; record main checks and coordinator consumer delta. No broad application tests.

# Output
Accepted bounded safe Tile extension with actual capacity/failure ownership evidence; cache writer can resume faithful accounting under original scope without another crate edit.
