# Deterministic UI test driver

`app/test_driver/driver.py` is a macOS-only, standard-library developer harness.
Its default mode replays JSONL against a small placeholder state model; native
AppKit testing is a separate `--native-platform-smoke` mode. Neither mode reads
SVG documents, renders SVG, or executes document content. In native mode the
open command's bounded document ID supplies the window title, and the current
synthetic key adapter accepts the key `a`.

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

## Native AppKit window smoke

`src/platform/platform_window.h` is the C-callable platform boundary. Its
opaque window belongs to the caller and must be created, polled, resized, and
destroyed on AppKit's main thread. Resize and scale payloads report backing
pixel dimensions and scale; pointer x/y are logical points from the content
view's top-left. Key payloads include the AppKit key code, modifier flags, and
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

The C Metal interface in `src/platform/platform_metal.h` accepts a borrowed
buffer valid through its synchronous call. Its macOS implementation uploads to
a shared Metal texture, renders that texture to the current CAMetalLayer
drawable, and waits for command completion with a five-second timeout. It then
reads the uploaded texture back and compares every byte, reporting the first
mismatch only. Metal, QuartzCore, and Cocoa are system frameworks; no external
dependency is used.

Run the native presentation, exact-readback, resize, and reset smoke with:

```sh
python3 app/test_driver/driver.py --native-metal-smoke \
  --output-dir .cache/WO-M1-02/native-smoke
```

This path needs the Release build and an active graphical session. It records
presented frame count, exact-readback result, resize/reset result, and elapsed
smoke time in its run directory. The elapsed value measures this smoke only;
it is not rendering throughput or memory usage.
