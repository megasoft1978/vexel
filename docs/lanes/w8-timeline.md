# W8 Animation Timeline

## Delivered

- Added an editor timeline panel for SVG documents with parsed animation
  tracks. It provides play/pause, loop, a scrub bar, a seconds readout, and a
  scrollable list of animated target elements and attributes.
- Space toggles playback; `[` and `]` step by 1/30 second. Clicking a track
  selects it and seeks to its begin time. Scrubbing pauses playback.
- Track names come from bounded source spans retained by the animation core.
  The panel uses UIKit widget painting and publishes labelled play, slider,
  loop, list, and row nodes together with the editor accessibility snapshot.
- Timeline samples use `raster_shape_render_at`. Playback requests at most 30
  redraws per second; static sharp tiles are not layered over animated frames.
  Indefinite tracks use a five-second browse window, bounded by the animation
  core's one-hour timeline limit.

## Verification

- Test-first focused run: `./dev t -k viewer-timeline` failed before the
  implementation because the newly registered timeline sources were absent.
- Final focused run: `./dev t -k viewer-timeline` passed; 1 CTest passed.
- `./dev t` — CTest 51/53; 16 Python tests passed. The only CTest failures
  were `index-replay` and `native-viewer-smoke` in this sandbox.
- `./dev t --all` — CTest 51/53; 69 Python tests passed. It reported the same
  two native failures.
- `git diff --check` — passed.

## Limits

- Native viewer presentation and animation playback were not verified: the
  sandbox cannot create a Metal device or command queue. Native viewer smoke
  also reports unavailable LaunchServices and `com.apple.hiservices` XPC.
- The semantic snapshot test verifies timeline labels and roles, but VoiceOver
  navigation and native control activation remain unverified. UDS behavior was
  not exercised; the timeline adds no UDS dependency.
- `raster_shape_render_at` runs synchronously on the viewer thread. Large
  animated documents may make playback and scrubbing wait for a full render.
