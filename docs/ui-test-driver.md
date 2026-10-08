# Deterministic UI test driver

`app/test_driver/driver.py` is a macOS-only, standard-library developer harness.
It replays a JSONL stream against a small placeholder state model; it does not
open a native window, read the named document, render SVG, or execute content.
The document ID is only a bounded label.

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
