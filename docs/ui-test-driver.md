# Deterministic UI test driver

`app/test_driver/driver.py` is a macOS-only, standard-library developer harness.
Its default mode replays JSONL against a small placeholder state model; native
AppKit testing is a separate `--native-platform-smoke` mode. Those driver modes
do not read SVG documents, render SVG, or execute document content. The
`svgworkbench --native-viewer-smoke <file.svg>` command separately exercises
the real viewer load, first presentation, drag, magnify, fit key, and Cmd+Q path.

## Invocation and limits

```sh
python3 app/test_driver/driver.py \
  --script tests/ui/mixed-commands.jsonl \
  --output-dir .cache/WO-BOOT-07/smoke \
  --seed 17
```

The script path must resolve inside the repository. The output directory must
be new, must not contain a symlink in its path, and must be under this
repository's `.cache/`. Invalid CLI, input, platform, or
output conditions exit with status 2 and a concise stderr message. The input
is UTF-8 JSONL, at most 8 MiB total, 10,000 commands, and 65,536 bytes per
line. Blank lines, duplicate or unknown keys, unknown commands, malformed JSON,
non-finite numbers, wrong types, and incorrect command order are rejected.
Booleans do not count as integers.

The exact command objects are:

| Command | Fields and limits |
| --- | --- |
| `open` | `document_id`: non-empty string, at most 256 UTF-8 bytes; exactly once and first |
| `resize` | integer `width`, `height`: each 1–4096; at most 16,777,216 pixels |
| `pointer` | `action`: `move`, `down`, or `up`; integer `x`, `y` inside current window |
| `key` | `action`: `down` or `up`; `key`: non-empty string, at most 128 UTF-8 bytes |
| `text_commit` | `text`: string, at most 4096 UTF-8 bytes |
| `frame` | integer `delta_ns`: 0–1,000,000,000; accumulated virtual time at most 1,000,000,000,000 ns |
| `progress` | integer `percent`: 0–100; `label`: string, at most 256 UTF-8 bytes |
| `accessibility`, `screenshot`, `save` | command name only |
| `quit` | command name only; exactly once and final |

The seed defaults to zero and must be an unsigned 32-bit integer. Dimensions
start at 320x240. A screenshot writes sequential `frame-NNNN.png` files in
8-bit RGBA, non-interlaced PNG format. Accessibility commands write sequential
`accessibility-NNNN.json` semantic trees with stable IDs, roles, names, values,
states, and bounds. Save writes `document.json` with deterministic placeholder
state. Each command emits one compact JSONL acknowledgment to stdout with its
index, virtual time, and relative output filenames.

Artifacts and acknowledgments are stable for the same script and seed. The
placeholder PNG only depicts a flat canvas and seed-derived accent; the
accessibility tree and saved state are not a real editor model. No wall-clock
time or sleep is used.

## Viewer navigation replay

`--navigation-replay` adds bounded `navigate`, generation-tagged
`index_progress`, and `cancel` commands. Navigate actions are `fit`, `wheel`,
`pinch`, or `pan`; each supplies finite `x`, `y`, and `value` fields whose
magnitude is at most 10,000. The replay mirrors the C model: scale is clamped
to 0.05–64, wheel factor is `exp(-value * 0.001)`, pinch factor is
`exp(value)`, and zoom preserves the world point under the cursor. Progress
generations cannot move backward; terminal generations cannot be reopened.
Cancel records a request and leaves the phase active until a subsequent
progress command reports cancellation. Active semantics include an enabled
Cancel indexing button; after a request it is disabled, and after cancellation,
completion, or failure it is omitted.

Run the frozen navigation baseline replay with:

```sh
python3 app/test_driver/driver.py --navigation-replay \
  --script tests/ui/navigation-replay.jsonl \
  --output-dir .cache/WO-M3-04/replay --seed 17
python3 tests/ui/verify_navigation.py --actual .cache/WO-M3-04/replay \
  --baseline tests/ui/navigation-golden
```

The legacy flat-canvas artifacts remain pinned by `manifest.json`. Each
navigation screenshot also emits a separate custom-rendered screen with the
transformed document shapes and progress/cancel overlay; those PNGs are pinned
by the additive `manifest-rendered.json`. The verifier checks both manifests.
Native window dimensions are backing pixels while AppKit cursor coordinates
are logical points. Events carry `backing_scale`; the navigation adapter
converts viewport and cursor values to backing pixels before fit or anchored
zoom; pointer-drag helpers use the same conversion. `--native-navigation-event-smoke` checks finite scale and cursor/delta
payloads using deterministic synthetic NSEvent inputs.

## Native SVG viewer smoke

Run the viewer's actual asynchronous load and presentation path with:

```sh
build/mac-debug/bin/svgworkbench --native-viewer-smoke tests/svg/shape/fixtures/rect.svg
```

The smoke waits for a document frame, injects a drag, trackpad magnify, `0` fit
key, and Cmd+Q, then reports its presented-frame count. It is a native AppKit
and Metal check and requires an active graphical Apple Silicon macOS session.
It is registered in CTest without a skip condition; headless environments fail
the test and should run it on the maintainer's Mac.

## Repeat replay and native Metal frame sequence

Run a fixed JSONL script repeatedly with:

```sh
python3 app/test_driver/driver.py --repeat-replay \
  --script tests/ui/mixed-commands.jsonl --repeats 20 \
  --output-dir .cache/WO-M1-03/replay
```

Each invocation reserves `run-NNNN` under the output root; each repetition has
its own `replay-NNNN` directory. Reusing the outer command reserves the next
number and leaves earlier output intact. `result.json` records normalized
command order, explicit frame deltas and accumulated virtual nanoseconds, each
artifact SHA-256, and the combined replay digest. The first run is compared to
every later run; any order or artifact hash difference fails the command. The
virtual clock advances only on `frame` commands. Wall time is not read by the
placeholder replay.

Native Metal replay submits the same deterministic 64x48 scalar tile for 20
frame steps using headless shared textures and Metal blit command buffers.
Every command buffer has a five-second completion bound; the call compares
every readback byte and calculates SHA-256 from the readback buffer. The native
process has a separate 180-second bound and reports ordered frames and the
repeated readback digest. It requires Apple Silicon macOS 13+ and Metal.
Windowed AppKit/Metal presentation is covered by the separate native smoke. The
replay does not test physical input, display timing, SVG rendering, throughput,
or RSS.

## Native AppKit window smoke

`src/platform/platform_window.h` is the C-callable platform boundary. Its
opaque window belongs to the caller and must be created, polled, resized, and
destroyed on AppKit's main thread. Resize and scale payloads report backing
pixel dimensions and scale; pointer x/y are logical points from the content
view's top-left. Key payloads include the AppKit key code, normalized modifier bits, and
bounded UTF-8 text. Close requests are events and leave the C-owned handle
valid until the caller destroys it. Polling pumps pending AppKit messages
without waiting and returns false when the queue is empty.

The driver reads `tests/ui/native-lifecycle.jsonl`, translates each open,
resize, key, pointer, and quit command into a bounded argument, and runs the
Release executable for native lifecycle smoke checks:

```sh
python3 app/test_driver/driver.py --native-platform-smoke --cycles 100 \
  --output-dir .cache/WO-M1-01/native-smoke
```

Each invocation reserves a new `run-NNNN` child beneath the requested output
root. Repeating the command preserves earlier smoke evidence and cannot replace
existing files. The root and every existing path component must remain under
`.cache/` and must not be a symlink.

The smoke executes those scenario actions, injects corresponding AppKit
key/pointer events, requests close, and destroys each window 100 times without
sleep-based synchronization. It
records stdout, stderr, the JSONL scenario, exact completed-cycle count, and
elapsed wall time. Synthetic AppKit events exercise event delivery and
normalization; they do not replace manual testing with physical mouse,
keyboard, trackpad, or display changes.

## Deterministic CPU tile and Metal presentation

`src/raster/tile.h` exposes `raster_tile_create` and `raster_tile_destroy`.
The caller owns the returned allocation and releases it with the destroy
function. Tiles are positive RGBA8 dimensions no larger than 4096 on either
axis; byte count is checked before allocation and equals `width * height * 4`.
The opaque test pattern is deterministic: R=`(17*x + 3*y) mod 256`,
G=`(5*x + 29*y) mod 256`, B=`(11*x + 7*y) mod 256`, A=255.

The C Metal interface in `src/platform/platform_metal.h` includes a synchronous
verification function that accepts a borrowed buffer, renders it to the current
CAMetalLayer drawable, waits for completion, and compares every byte on
readback. The interactive viewer uses `platform_metal_present`, which reuses
its device, queue, pipeline, and size-matched texture and submits presentation
without readback verification. Metal, QuartzCore, and Cocoa are system
frameworks; no external dependency is used.

Run the native presentation, exact-readback, resize, and reset smoke with:

```sh
python3 app/test_driver/driver.py --native-metal-smoke \
  --output-dir .cache/WO-M1-02/native-smoke
```

This path needs the Release build and an active graphical session. It records
presented frame count, exact-readback result, resize/reset result, and elapsed
smoke time in its run directory. The elapsed value measures this smoke only;
it is not rendering throughput or memory usage.

## Native task queue smoke

The native task smoke uses the current Debug executable. It checks queue bounds,
injected allocation failures, queued and running cancellation,
generation-safe publication, the owner-quiescence assertion, and repeated
shutdown/join cycles. Capacity is 1–64 tasks with 1–4 workers. Callback context
is borrowed and stays alive through completion or cancellation. Callbacks poll
their atomic cancellation token and return promptly. Old-generation results
are discarded. Shutdown cancels, wakes, and joins workers before releasing
storage; callbacks that ignore cancellation can delay join. The owner stops new
API callers and waits for admitted calls to finish before destroy.
