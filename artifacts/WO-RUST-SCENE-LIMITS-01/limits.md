# WO-RUST-SCENE-LIMITS-01 limits table

Measured with `rust/crates/vexel-app/tests/scene_limits_measure.rs` (ignored test,
`VEXEL_LIMITS_FILES`), release, macOS CoreText provider linked, 128x128. Diagnostics came from
temporary, reverted instrumentation of the verdict site and the text-command counter.

| limit | site (main, after WO-RUST-PIPELINE-PERF-01) | before | after | unit | legacy budget | measured need | changed |
|---|---|---|---|---|---|---|---|
| `MAX_REFERENCE_IDS` | vexel-svg-reference/src/lib.rs:13, build.rs:230 | 65,536 | 262,144 | ids | table cap (C: 65,536) | chua 100,088; ideogram 144,217 | yes |
| `REFERENCE_SLOT_CAPACITY` | vexel-svg-reference/src/lib.rs:17 | 131,072 | 524,288 | u32 slots | - | power of two >= 2x ids | yes |
| cumulative `use` reference bytes | vexel-svg-decode/src/use_expansion.rs:99 | 1,048,576 | 16,777,216 | bytes (counter) | C: 1 MiB | use_t1 1.60 MB (tripped at byte 648,183); use_t2 16.25 MB | yes |
| per-reference bytes | use_expansion.rs:99 | 65,536 | 65,536 | bytes | - | - | no |
| `use` nesting | use_expansion.rs:71 | 16 | 16 | levels | - | - | no |
| `use` instances | use_expansion.rs:71 | 65,536 | 65,536 | instances | - | use_t2 178,589 | no (see below) |
| `Limits.shapes` | vexel-scene/src/types.rs:120 + `Limits::effective` clamp (types.rs:398-403), dash_validate.rs:8, raster_job_views.rs:84 | 65,536 | 65,536 | shapes | C: 65,536 elements | use_t2 178,589 | no, blocked |
| `DOCUMENT_TEXT_COMMAND_LIMIT` | vexel-scene/src/request.rs:230 | 4,194,304 | 4,194,304 | commands | - | text_t1 15,849,329 (see below) | no (assertion added) |
| edges, per-path commands, elements, text bytes, glyphs (1,048,576 / 65,536 per run), transform ops, numbers | vexel-scene types.rs, vexel-svg-media text caps | unchanged | unchanged | - | legacy | - | no |

## Why shapes/instances were not raised
`Limits::effective()` clamps every field to `Limits::default()`, so the value set in
`compose/decode.rs` cannot exceed 65,536. Verified: with `Limits.shapes` = 262,144 and instances =
262,144, use_t2 still ended `Limit(Shapes)` at byte 3,683,732 (about 65,536 uses). Raising it needs edits to
`vexel-scene/src/types.rs` (default) plus the two hardcoded `65536` shape checks in
`dash_validate.rs` and `raster_job_views.rs`, outside this order's allowed paths. Both experiments were
reverted. Value needed for use_t2: 178,589 shapes and instances (262,144 would cover it).

## Text
text_t1 with the real provider, limits temporarily lifted: 15,849,329 text commands (10,652 text
elements), then `Limit(Render)`. With only the command limit lifted it stops at `Limit(Edges)` after
4,396,474 commands (edges 4,194,304 is a legacy budget, non-goal). Need exceeds 8,388,608, so the
limit stays 4,194,304 (AC4). text_t2 with limits lifted reached 70,320,487 commands and `Limit(Edges)`; the
glyph cap was not reached first. With real limits both stop at `TextCommands`, byte 235,947.

## Worst-case memory
- ids: table is `u32` slots: 524,288 x 4 = 2 MiB (was 512 KiB), allocated once, and only after the id
  count passed (N+1 ids refuse in the counting phase, before any table allocation).
- reference bytes: a `usize` counter; 256 uses of a 64 KiB target (the full 16 MiB budget) compose at 12.5 MiB
  peak RSS, so nothing is allocated per byte.
- text commands: 4,194,304 x 56 B = 224 MiB, const-asserted <= 512 MiB (request.rs).
- shapes: unchanged.

Peak RSS (release test binary, `/usr/bin/time -l`, includes harness and file read):

| synthetic worst case | verdict | peak RSS |
|---|---|---|
| ids = 262,144 (4.3 MB) | ok | 221 MB |
| ids = 262,145 | LimitDecode | 70 MB |
| 256 uses x 64 KiB target (16 MiB budget exactly) | ok | 12.5 MB |
| 257 uses x 64 KiB target | LimitDecode | 12.1 MB |
| 65,536 uses x 91 B target | ok | 199 MB |

Compile-time assertions: `REFERENCE_SLOT_CAPACITY.is_power_of_two()`, `>= 2 * MAX_REFERENCE_IDS`
(vexel-svg-reference/src/lib.rs), and `DOCUMENT_TEXT_COMMAND_LIMIT * size_of::<TextCommand>() <= 512 MiB`
(vexel-scene/src/request.rs). Shape assertion not added (shapes not raised).
