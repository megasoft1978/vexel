# WO-M4-06 handoff

WO-M4-06 revision 4 is implemented on `wo/WO-M4-06`. The SVG omitted
`stop-color` default now renders as black and has a focused regression plus a
librsvg fixture. The amended copy limits are 64 KiB per referenced subtree and
1 MiB combined per render job. Current renderer source is
`e53c760436c0d16afb9b15f7e8adaf6967dc9a2a`; README-only follow-up
`3396d8109223de9018956104b066d8d0a5255ace` documents the default.

Machine gates pass: Debug CTest 12/12, `./dev verify` Python 199/199 and native
smoke/replay, ASan/UBSan CTest 12/12, Release build, 1,000,000 sanitized fuzz
cases, 35 librsvg fixtures, and 58 corpus files / 116 trials. Corpus outcomes
remain 2 previewable, 56 unsupported, no limits/crashes; this is still the
open/index harness. Independent read-only review and merge are next.

After merge, continue with WO-M4-06b to wire the renderer into the viewer and
corpus harness, then M4-07 (skip the bake-off if it costs more than one WO) and
M4-08 fidelity gate packet.
