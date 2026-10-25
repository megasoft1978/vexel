# ADR-0036: Cartoon pipeline v2 (draft plan)

Status: proposed. Source: "Ember Call" production test (8 shots, 116 s animatic).

## Goal
Make a 2-5 minute 80s-style cartoon with far fewer tool calls, far more motion
and effects, and fast iteration. Speed is a top-level requirement, not a follow-up.

## Findings from the test
- Motion was stiff: characters were 1-2 rigid cut-out layers, camera and
  parallax carried the shot. Cause: rigging is manual (hand polygons, no skeleton),
  so lanes skipped limb animation.
- Cost of one shot: 50-60 s build, of which 20-45 s is flushing 66-231 tracks;
  peak RSS 0.8-1.3 GB; preview tick 64 ms (shot 1) vs 40 ms target.
- Pipeline needed ~10 external scripts (trace, segment, inpaint, rig, mix, mux).

## Workstreams

### A. Motion engine (more fluid, more effect)
1. Easing presets + cubic-bezier per key (`ease: in|out|inout|back|elastic|bounce`).
2. Noise/jitter track (seeded): shake, flicker, hand-drawn boil, wind sway.
3. Skeletal rig: bones with pivots, parent chain, per-part weights as rigid
   parts (no mesh). `rig_create`, `rig_pose`, `rig_clip` (walk/run/talk cycles as clips).
4. Motion library: reusable clips (loop, ping-pong, blend) applied to rigs.
5. Camera: first-class `camera` (pan/zoom/rotate/shake/rack-focus), one track
   set instead of wrapping the world group.
6. Speed lines, directional/motion blur, glow/bloom, chromatic split, film grain,
   halftone, VHS scanlines as built-in effect presets (filter + blend fragments).
7. Mouth swap primitive: viseme set per character, driven by audio envelope.
8. Eye clipping (clip-path from eye shape), shape-keyed blink.
9. Smear frames / squash-stretch helper for fast moves.

### B. Authoring friction (fewer calls)
1. Lift limits: rect/ellipse 512 px cap, path `d` 4096 B, `element_group` >35
   children, linecap/fill-rule/short hex via `element_add`, gradient ellipses,
   renaming groups, `use_add` scale/offset order, trs scale 0.001.
2. Import a `.vexel` rig/scene into another document (`document_import`), with id remap.
3. `scene_apply` / one-shot: whole scene (tree + tracks) from one JSON, no batches of 20-40.
4. Tracks without prior `native_timeline_set_duration`; hidden parent wins over child visibility.
5. Trace-assist inside the app: `image_trace` (k-means quantise, contours, Bezier fit)
   and `image_segment` (grabCut-like part cutting), `inpaint` for underlays.
   Open question: link OpenCV-free Rust implementation vs helper process.
6. Fix fragment undefined-filter error message; reconcile docs/mcp.md blend-mode text.

### C. Speed (top priority)
Targets on the real cartoon shots, 900 px preview: tick <=25 ms (now 64 ms),
track flush of 200 tracks <=2 s (now 37-44 s), open <=30 ms, peak RSS <=600 MB (now 1.3 GB).
1. Track flush: batch path is O(n^2)-ish; add bulk `tracks_set` with columnar
   payload and one timeline rebuild. Measure first.
2. Layer cache: cache rasterised groups; camera/transform-only changes re-composite
   bitmaps instead of re-rasterising paths (the 49 ms of the shot-1 tick).
3. Incremental decode: only changed elements re-decode on each tick.
4. Memory: arena for path points, drop decoded copies after raster cache fill.
5. Export: pipeline frame render and encode, tile parallelism, reuse layer cache.
6. GPU route policy for many small clipped shapes; keep CPU as reference.
7. Bench gate: `cartoon` bench cases per shot scene, baseline file, CI informational.

### D. Audio (local models, in product and MCP)
Decision needed (see Questions): in-process vs helper process.
- Timeline audio tracks (voice, sfx, music) with in/out, gain, fade, loop, ducking.
- Export mux: audio into mp4/mov (AVFoundation), mix down with limiter.
- MCP: `audio_generate_voice` (Kokoro, voice per character), `audio_generate_sfx`,
  `audio_generate_music`, `audio_add_track`, `audio_set_cue`, `audio_envelope`.
- Lip-sync: envelope -> viseme track on a rig mouth, automatic.
- Models stay local; first run downloads with consent; no network at render time.

### E. Quality pass on Ember Call (after A-C land)
Rebuild with skeletal rigs and motion library: running, turning, talking limbs,
smear frames, camera moves, speed lines, glow, parallax depth, particle bursts.
Restore shot 5 (montage). Fix village plate smear, shot 9 face and horizon seam.
Full audio from workstream D inside Vexel, no external mixing.

## Order and lanes (one writer per module)
1. Baseline: profile harness on all 8 shots, record numbers (C, no code).
2. Lane 1: B1+B4+B6 limits and fixes. Lane 2: C1 track flush + C7 bench gate.
3. Lane 3: A1+A2+A5 easing, noise, camera. Lane 4: C2+C3 layer cache and incremental decode.
4. Lane 5: A3+A4+A7+A8 rig, clips, visemes, eye clip. Lane 6: B2+B3 import and scene_apply.
5. Lane 7: D audio timeline + mux. Lane 8: D generation + lip-sync. Lane 9: B5 trace/segment.
6. A6+A9 effects presets. Then E.
Each lane: ADR note, tests, security review for anything parsing input; never weaken goldens.
Respect main.rs/preview.rs 800-line cap (new code in new modules).

## Questions for owner
1. Audio generation: in-app (links Python/MLX or Rust ONNX, heavy) vs optional
   helper process that Vexel spawns (recommended: keeps app small, models swappable).
2. Trace/segment: native Rust (recommended, no OpenCV dependency) vs helper.
3. Rig model: rigid-part skeleton only (recommended) vs add mesh deformation later.
