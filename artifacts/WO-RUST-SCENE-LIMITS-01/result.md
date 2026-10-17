# WO-RUST-SCENE-LIMITS-01 result

Candidate commit 763e84d8 on branch worktree-agent-a1166d62f8219823f (this file amended in a follow-up commit).

| file | before | after | remaining ceiling |
|---|---|---|---|
| generated-use_t1 | limit_decode (references, byte 648,183) | ok, 255 ms, 91 MB RSS | none |
| generated-use_t2 | limit_decode | limit_decode (Shapes at byte 3,683,732) | `Limits::effective` clamp 65,536 shapes in vexel-scene (out of scope); needs 178,589 |
| chuaattractormodified | limit_decode (ids) | unsupported `<translate>` at byte 2053, 158 MB | V1 feature gap |
| ideogram-human-chromosome-20 | limit_decode (ids) | unsupported `<11px>` at byte 20,677,771, 131 MB | V1 feature gap |
| generated-text_t1 | limit_decode (TextCommands) | unchanged | needs 15,849,329 commands (> 8,388,608 cap), then Edges and Render limits |
| generated-text_t2 | limit_decode (TextCommands) | unchanged | >70.3M commands, then Edges |

Changes (each proven by a failing test added first):
- `MAX_REFERENCE_IDS` 65,536 to 262,144, `REFERENCE_SLOT_CAPACITY` 131,072 to 524,288 (2 MiB);
  `corpus_measured_id_counts_are_indexed`, plus scene_limits.rs through `compose_svg`.
- Cumulative `use` bytes 1 MiB to 16 MiB; `use_reference_budget_admits_the_measured_generated_use_t1_shape`,
  256/257 boundary tests. Existing 1 MiB pins updated to the 16 MiB boundary (original_references.rs, original_m4_boundaries.rs).
- Text command limit unchanged; const assertion added.

Sweep (external, release 128x128): ok 10 to 11, no regressions, no rss_exceeded/crash; in-repo 24/24 ok unchanged.
Tests: 851 passed, 0 failed. The frozen fmt check fails on pre-existing unformatted code in vexel-scene and
vexel-svg-reference tests; files added here are rustfmt-clean.
