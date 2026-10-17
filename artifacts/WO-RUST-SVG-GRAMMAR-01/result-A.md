# WO-RUST-SVG-GRAMMAR-01 item (A): compact-dot path grammar

Base: worktree branch `worktree-agent-a11f8bdccc3ece5c0` (vexel main, macOS, Rust only).
Cargo: `CARGO_HOME=/Users/megasoft78/Desktop/Freelance/vexel/.cache/cargo`, `--offline --locked`.
Build output goes to `build/rust` (from `.cargo/config.toml`).

## Rule

SVG path grammar: a `.` that directly follows a number with no separator starts a new number.
`1.2.3` = `1.2`, `.3`. `1.` is a valid number (SVG 1.1/2 `fractional-constant ::= digit-sequence "."`),
so `1..2` = `1.`, `.2` and was already accepted before this change. `.` after a complete group
starts an implicit repeat of that command. The legacy C parser rejected these; that was the bug.

## Code changes

- `src/parser.rs`: `begin_group` accepts `b'.'` as a number start after a number (the missing
  check at the old ~478-480 site). Removed the sticky `compact_dot_boundary` flag and its
  check (old ~459-462). The flag was only ever a rejection of implicit repeats after any
  compact dot.
- `src/reader.rs`: removed `compact_dot_boundary` and `integer_digits` (both only fed the removed
  flag). No other behaviour touched.

## Failing-first record (before the fix)

Command: `cargo test -p vexel-svg-path --test compact_dot_grammar`
- `inkscape_and_illustrator_compact_dot_strings_match_spaced_forms`: `M0 0l-.1.21.1.55z` -> `Diagnostic { kind: Path, offset: 11 }`
- `compact_dot_splits_fraction_into_two_numbers`: `M0 0 L.2.3.4.5` -> `Diagnostic { kind: Path, offset: 10 }`
- `compact_dot_begins_an_implicit_repeat_after_a_complete_group`: `M0 0 L1.2.3 4 5` -> `Diagnostic { kind: Path, offset: 6 }`
- `huge_compact_dot_counts_are_bounded_and_exact`: `Diagnostic { kind: Path, offset: 10 }`
- `compact_dot_free_inputs_match_pre_change_parser_digest`: `actual digest 0x922d0b63cab73464` (the digest was recorded from the pre-change parser, then pinned as `PRE_CHANGE_DIGEST`)
- Passing before and after, as negatives/controls: `malformed_compact_dot_forms_still_fail`, `incomplete_compact_pair_reports_the_group_error`, `compact_dot_after_arc_radius_begins_a_new_number`, `compact_dot_detector_flags_only_compact_forms`.

## Pins changed (old -> new, with justification)

1. `rust/fixtures/legacy/path/baseline-golden.txt`, CASE `grammar-7` (`M0 0 L.2.3.4.5`):
   `ERROR Path 10` -> `OK` with 3 edges, 3 segments, 6 numbers. Spec: `L.2.3.4.5` = `L 0.2 0.3 0.4 0.5`, two complete pairs.
   `parity.rs` assertions and all other golden blocks are unchanged.
2. `rust/crates/vexel-svg-path/tests/number_separator.rs`, `integer_fraction_adjacency_rejects_second_decimal_across_entry_points`
   renamed to `integer_fraction_adjacency_splits_second_decimal_across_entry_points`.
   `M0 0 L1.2.3 4 5`: `Err(Path @13)` -> `Ok`, 6 numbers, 3 segments. Checked against `parse_path`,
   `parse_path_controlled` and the session step loop, and against the spaced form `M0 0 L1.2 .3 4 5`.
   Correction to the brief: `4 5` is a complete implicit L pair, not an incomplete one, so the spec result is success.
3. `number_separator.rs`, `compact_arc_endpoint_allows_an_implicit_repeated_arc_group` (second half, `compact_radius`):
   `M0 0 A2.3.4 0 0 1 4 5 2 3 0 0 1 6 7`: `Err(Path @6)` -> `Ok`, 16 numbers, 3 segments.
   Spec: radii 2.3 and .4, then a valid implicit arc. This pin was not in the brief's list; it is the same legacy bug.

Not changed, but required: `rust/crates/vexel-svg-scalar-index/tests/original_suites.rs:166` (see Gaps).

Unchanged and passing: all other `number_separator.rs` negatives (`M.4..6`, `M1e 0`-style cases), all of `controlled.rs`, `path.rs`, and the parity test bytes for every other case.

## Tests

- New `tests/compact_dot_grammar.rs` (289 lines, 9 tests): sample strings vs spaced forms, splits, implicit repeats, arc radius, malformed forms (`1..2` is listed as valid, not malformed), `L.2.3.4` incomplete pair (offset 12, end of input), huge counts (20,000 numbers accepted, 20,001 odd rejected, `max_numbers` cap yields `Limit`), seeded property test (4096 inputs, generator never emits a compact-dot form, checked by `has_compact_dot`), detector test.
- `cargo test -p vexel-svg-path --locked --offline`: all green (9, 9, 13, 4, 1, 13 tests across the targets).
- Workspace (`cargo test --offline --locked --no-fail-fast --workspace`): 1953 passed, 1 failed (see Gaps).
- `python3 scripts/check_rust_fmt.py`: pass.

## Corpus verdicts (`vexel-corpus run --manifest ~/Desktop/Freelance/vexel-corpus-real/manifest.json --pins rust/tools/vexel-corpus/pins.json --out <dir> --only <id>`, read-only corpus)

| id | before | after |
|---|---|---|
| commons-france-osm-mapnik-svg | xml: path data at byte 28855 | unsupported: `<overflow>` at byte 289 (V1 unsupported element) |
| commons-germany-districts-municipalities-location-map-2013-svg | xml: path data at byte 474 | limit_decode: "document exceeds a decode safety limit" |

The path blocker is removed for both. Neither is `ok`. Mapping `ErrorKind::Limit(_)` drops the limit kind, so the germany limit is not identified. Reports: `/private/tmp/svgpath-grammar/before/report.md`, `/private/tmp/svgpath-grammar/after3/report.md`.
Before binary: built from the unchanged tree. After binary: fresh `build/rust/.../vexel-corpus` (an earlier copy from `target/` was stale and was discarded).

## Gaps

- BLOCKER for AC 10 (workspace green): `rust/crates/vexel-svg-scalar-index/tests/original_suites.rs:166`,
  test `original_bounds_diagnostic_full_svg_sources_reject_paths_with_absolute_offsets`. The "frozen" peer-rejection list contains
  `M0 0 L1.2.3 4 5`, which is now spec-valid. The fix is to remove that one string from the list, or move it to the accepted side.
  That file is outside this WO's allowed paths, so I did not edit it. Needs coordinator approval.
- Germany: limit kind unknown (see above). Only item (C)'s kind mapping could expose it.
- France: `<overflow>` is an unsupported element, not in scope of item A.
- The number/float arithmetic for fractions is the existing reader's (`0.3` from `.3` is `0.30000000000000004`, same as the spaced form).
