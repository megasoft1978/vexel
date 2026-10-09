# SVG stress generator and oracle renderer

`stress_svg.py` uses only Python's standard library. Its explicit 64-bit seed,
independent semantic controls, and exact byte target produce deterministic
documents. It writes fixed 64 KiB padding chunks to a temporary file, hashes as
it writes, fsyncs, then installs with an atomic no-overwrite hard link. A rerun
verifies identical existing outputs without replacing them; mismatches fail
without changing the destination. Failures remove temporary files.

The API limits are: total output 1,000,000,000 bytes; 1,000,000 objects;
5,000,000 path commands; 128 nested groups; 16,000,000 embedded payload bytes;
and an unsigned 64-bit seed. Semantic content is generated before opening the
destination so an invalid or semantically oversized request is rejected
without creating an output. The CLI restricts generated artifacts to the
repository's ignored `.cache/` tree.

The matrix creates exact 10,000,000-byte object-heavy, 100,000,000-byte
path-heavy, and 1,000,000,000-byte mixed SVG files. Generated documents stay in
`.cache` and are never committed. The JSON report records per-file SHA-256,
semantic counts, seed, duration, and machine metadata. `--verify-report` checks
hash and size offline and never changes the input files.

`render_oracle.py` is an optional local baseline tool. On the first run it
requires the pinned renderer version, renders each of the 24 corpus fixtures
at 256x256, then records source/PNG hashes and renderer/license/environment
metadata. Later runs verify that lock and its bytes without rendering or
rewriting goldens. The committed lock is checked offline by the normal test
suite; no renderer is needed by builds or tests.

`index_bench.py`, dispatched by `./dev index-bench`, generates separate 10 MiB
simple-element and 100 MiB map-like inputs under `.cache/index-bench/`. It runs
five first-use and ten warm trials through `svg_index_bench`, records source
read, first worker-rendered preview, full validation/index time, maximum owner
step, record bytes, RSS, raw trial values, toolchain metadata, and input hashes.
The cold/warm labels do not flush the macOS filesystem cache.
