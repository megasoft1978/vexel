# Unsupported causes (WO-RUST-CORPUS-UNSUPPORTED-01)

## in-repo: the 7 files `unsupported` at the order's reference report

| file | cause at reference report | verdict/cause at current HEAD (before) | verdict after |
|---|---|---|---|
| clip-path | unsupported in V1: clip path | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| image-embedded-data | unsupported in V1: image (<image> at byte 208) | unsupported: unsupported in V1: image (<image> at byte 207) | ok: 128x128 frame, 0 disclosure(s) |
| stress-text-collection | unsupported in V1: text (<text> at byte 219) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| style-opacity-transform | unsupported in V1: opacity group | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| text-basic | unsupported in V1: text (<text> at byte 199) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| text-positioning | unsupported in V1: text (<text> at byte 205) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| text-spans | unsupported in V1: text (<text> at byte 199) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |

### in-repo: per-file verdict changes (before -> after)

- image-embedded-data: unsupported -> ok

previously `ok` files no longer `ok`: 0

## external: the 7 files `unsupported` at the order's reference report

| file | cause at reference report | verdict/cause at current HEAD (before) | verdict after |
|---|---|---|---|
| generated-cad_t1 | unsupported in V1: text (<text> at byte 840302) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| generated-embedded_t1 | unsupported in V1: image (<image> at byte 894) | unsupported: unsupported in V1: image (<image> at byte 893) | ok: 128x128 frame, 0 disclosure(s) |
| generated-inkscape_t1 | unsupported in V1: filter (<filter> at byte 623) | unsupported: unsupported in V1: marker (<marker> at byte 688) | timeout: killed after 60 s wall-clock timeout |
| generated-science_t1 | unsupported in V1: text (<text> at byte 1374260) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| generated-text_t1 | unsupported in V1: text (<text> at byte 944) | limit: document exceeds a safety limit | limit: document exceeds a safety limit |
| natural-earth-world-map-blank | unsupported in V1: clip path | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |
| natural-earth-world-map-countries | unsupported in V1: text (<text> at byte 241676) | ok: 128x128 frame, 0 disclosure(s) | ok: 128x128 frame, 0 disclosure(s) |

### external: per-file verdict changes (before -> after)

- generated-embedded_t1: unsupported -> ok
- generated-inkscape_t1: unsupported -> timeout

previously `ok` files no longer `ok`: 0


## Residual blockers after the order

- generated-inkscape_t1 (`<marker>` at byte 688 before): markers and the first blocking
  construct are fixed; the file now composes (34,715 `stroke="url(#g)"` shapes are
  disclosed as gradient paint). Sweep verdict is `timeout` (about 160 s against the
  60 s limit). Cause: scene validation per committed shape request is quadratic for
  shapes that are not root-batched (4000 shapes 1.5 s, 8000 shapes 6.4 s); `cad_t1`
  (45,524 shapes, 59 s) is in the same family. Non-goal of this order (render/decode speed).
- generated-text_t1 (text at byte 944 before): text renders; the file stops at
  `LimitKind::TextCommands` at byte 235947 (`DOCUMENT_TEXT_COMMAND_LIMIT`, `vexel-scene`,
  a protected crate). Verdict `limit`.
- generated-embedded_t1, image-embedded-data: `ok` (data-URI PNG, 5,388 and 1 images).
