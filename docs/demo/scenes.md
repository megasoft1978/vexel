# Demo scenes (MCP-driven)

Three deterministic 512x512 scenes, built over the MCP stdio server
(`vexel --mcp-preview`) by stdlib-only Python scripts. All write to
`/private/tmp/vexel-demo/` (granted dir) and unlink existing outputs first.

Shared code: `scripts/scene_common.py` (MCP client with a rolling 560-cost/min budget guard,
`Builder`, PNG diff, export and ffprobe helpers). Scenes are declared as a virtual tree and sent as
`batch` requests: gradients/clips, leaves (explicit ids), `element_group` by nesting level
(deepest first), attributes/transforms, then all tracks with bulk `keys[]`.

Flags (all three scripts): `--fast` (default; no throttle sleeps, the budget guard still
applies) and `--paced` (0.35 s between calls, for a screen recording to watch the scene build).
The release binary is taken from the worktree build, else from the main checkout.

```sh
python3 -I scripts/scene_walk_home.py [--fast|--paced] [--legacy]
python3 -I scripts/scene_logo.py      [--fast|--paced] [--no-loop-variant]
python3 -I scripts/scene_loader.py    [--fast|--paced] [--no-alpha-variant]
```

Dev aid: `SCENE_PROBE=0.5,1,2 python3 -I scripts/scene_logo.py` renders extra stills
(`<name>_p<sec>.png`) and skips save/exports.

## Results (release build, loaded machine)

| scene | ticks / loop | shapes + groups (tree_get) | tracks (keys) | requests (cost) | build | export |
| --- | --- | --- | --- | --- | --- | --- |
| walk_home | 96 / loop | 229 + 36 = 265 | 136 (1845) | 18 (70) | 13-16 s flush | mp4 96 frames, 4.0 s, 54 s |
| logo | 72 / once | 56 + 19 = 75 | 67 (360) | 15 (36) | 0.7 s | mp4 72 frames, 3.0 s, 23 s |
| logo_loop | 96 / loop | 56 + 19 = 75 | 67 (427) | 15 (36) | 0.8 s | mp4 96 frames, 4.0 s, 32 s |
| loader | 48 / ping_pong | 83 + 12 = 95 | 46 (284) | 15 (35) | 0.4 s | mp4 94 frames, 3.92 s, 33 s |
| loader_alpha | 48 / ping_pong | 80 + 12 = 92 | 46 (284) | 15 (35) | 0.4 s | mov HEVC+alpha 94 frames, 22 s |

All counts are well under 400 elements. Per call: `batch` p50 about 42 ms (large batches:
tracks batch of 136 tracks / 1845 keys 13 s on a loaded machine; the 8-op gradient batch and
leaf batches 0.1-0.5 s), `render_png` 0.3-0.4 s (0.6-0.9 s for walk_home), `save_as` 10-20 ms,
`export_animation` 22-55 s (synchronous, dominated by encoding; machine was busy with other lanes).
ffprobe: all mp4/mov streams are 512x512, 24 fps, frame counts as in the table
(`nb_read_frames`); the alpha mov is `hvc1` and AVFoundation reports
`ContainsAlphaChannel = 1` (ffprobe shows `yuv420p` because it only decodes the base layer).
Ping-pong export holds 48 + 46 frames (endpoints not repeated).

## 1. walk_home

Batch port of the per-call builder. `--legacy` runs `scripts/scene_walk_home_legacy.py`
(the original per-call geometry; tracks use bulk `keys[]` by default, `--per-key` restores one
request per key, about 11 minutes) and writes `walk_home_legacy.*`.

Equivalence check: `render_png` at 0 s and 2 s, legacy vs batch, compared with
`scene_common.png_diff`: **max channel diff 0, 0 differing pixels** at both times.
Requests: legacy 453 (123 s at 0.108 s throttle) vs batch 18 (cost 70).

Outputs: `walk_home.vexel`, `walk_home_a.png` (0 s), `walk_home_b.png` (2 s), `walk_home.mp4`.

## 2. logo

Kinetic VEXEL reveal. Text decision: `text_add` only gives the platform default regular-weight
font (`attribute_set` rejects font-family/weight and text stroke), too thin for a logo, so the
five letters are bold filled polygon paths (one group per letter: extruded shade, gradient face,
white flash overlay). `text_add` is used for the tagline.

Motion: letters scale 0 -> 1.15 (`ease_out`) -> 1 (`ease_out_back` settle) with rotate/translate and
opacity, staggered 4 ticks; underline wipes in via scaleX from its left pivot with a glow head; 12
sparks (halo + core groups) in a group rotating 360 degrees, opacity pings; flash sweep over
letters; tagline fades in; rings and bokeh breathe. `logo_loop` (96 ticks) adds an `ease_in_back`
exit and a right-pivot underline wipe-out; tick 95 equals tick 0 (blank).

Gotcha found: a scale of exactly 0 makes `render_png`/export fail ("element or attribute outside
the supported SVG subset after the timeline is applied"); the scene uses 0.001.
Gotcha: ellipse/rect/path creation must lie inside 512x512 (glow ellipses are sized to fit).

Outputs: `logo.vexel`, `logo_a.png` (1.0 s), `logo_b.png` (2.75 s), `logo.mp4`; `logo_loop.vexel`,
`logo_loop_a.png` (1.5 s), `logo_loop_b.png` (2.5 s), `logo_loop.mp4`.

## 3. loader

Dashboard card: gauge (cubic-path circle; `attribute_set` adds `stroke-dasharray`,
`stroke-dashoffset`, `stroke-linecap`, then a `stroke-dashoffset` scalar track animates the arc),
11 percent labels toggled by step `visibility` tracks, 12-dot ring (opacity chase + group rotate), 8
bars (`height` and `y` scalar tracks, `ease_out_back`, 3-tick stagger) and three tiles whose fill
`width` is animated. `loader_alpha` omits the full-bleed background so the area outside the card
is transparent; it exports `format mov, codec hevc, alpha true`.

Outputs: `loader.vexel`, `loader_a.png` (0.5 s), `loader_b.png` (1.5 s), `loader.mp4`;
`loader_alpha.vexel`, `loader_alpha_a/b.png` (RGBA), `loader_alpha.mov`.
