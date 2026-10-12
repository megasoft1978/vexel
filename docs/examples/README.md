# Un cuore per due: supplied demo outputs

These three files were copied byte-for-byte from the owner-authorized, read-only external project directory `/Users/megasoft78/Desktop/Freelance/animazioni/animazione_output/vexel-deep-test`. No original scans, recordings, or individual frame captures were imported. Source and destination bytes and SHA-256 hashes were compared; machine evidence is in `artifacts/WO-UI-DOCS-01/asset-verification.json` at the repository root.

The SVG is a vector trace/reconstruction with transform articulation and squash/stretch. It does not establish exact original-drawing fidelity, bone skinning or path morphing. The owner supplied the outputs for this demo; upstream artwork authorship and reuse rights are not established here. Vexel's MPL-2.0 software license does **not** cover the supplied artwork. Do not infer permission for other artwork use.

| Asset | Dimensions | Bytes | SHA-256 |
| --- | --- | ---: | --- |
| [un-cuore-per-due-vector-rig-v2.svg](un-cuore-per-due-vector-rig-v2.svg) | 1034 × 1521 | 4228716 | `acac65077488157cd232776eaea555896157362f98e2809258380259b267751e` |
| [un-cuore-per-due-vector-rig-preview-v3.gif](un-cuore-per-due-vector-rig-preview-v3.gif) | 348 × 512 | 15690926 | `52651c529af23e499da7047749b91dac7c1801950acedbccb81eddc4f2db6871` |
| [v3-contact-sheet-8fps.png](v3-contact-sheet-8fps.png) | 624 × 654 | 328710 | `4db2db536a515387742ccfd5552559d7b463775881a7f535f5798d0dc134bb0b` |

## Timing and animation metadata

The SVG uses `viewBox="0 0 1034 1521"` and 12 named `animateTransform` tracks, all `calcMode="spline"` with explicit key times/splines and `fill="freeze"`. Mouse tracks begin at 0 seconds; cat tracks begin at 4.4 seconds. The longest mouse volume track repeats 3 × 3.67 seconds, so the SVG scene spans **11.01 seconds**.

The GIF is 348 × 512 pixels, contains **88 frames**, each encoded at **120 ms**, and loops indefinitely (`loop=0`). Its encoded loop lasts **10.56 seconds**. The 8 fps workflow label and contact-sheet name do not change the encoded 120 ms frame timing. The SVG and GIF durations are distinct.

| Track ID | Transform | Begin | Duration | Repeats |
| --- | --- | --- | --- | --- |
| `anim-mouse-body` | scale | 0s | 11s | 1 |
| `anim-mouse-legs` | rotate | 0s | 11s | 1 |
| `anim-mouse-heart` | rotate | 0s | 11s | 1 |
| `anim-mouse-head` | rotate | 0s | 11s | 1 |
| `anim-mouse-volume` | scale | 0s | 3.67s | 3 |
| `anim-mouse-root` | translate | 0s | 11s | 1 |
| `anim-cat-tail` | rotate | 4.4s | 6.6s | 1 |
| `anim-cat-body` | scale | 4.4s | 6.6s | 1 |
| `anim-cat-legs` | rotate | 4.4s | 6.6s | 1 |
| `anim-cat-head` | rotate | 4.4s | 6.6s | 1 |
| `anim-cat-volume` | scale | 4.4s | 2.2s | 3 |
| `anim-cat-root` | translate | 4.4s | 6.6s | 1 |

## MCP workflow evidence and limitations

The task handoff records successful MCP sampled-frame rendering and a failure of built-in full-page animation export. This imported preview GIF is an external project output, not a successful built-in-export result. No MCP/exporter source is changed by this work order.

Element handles refer to an edit snapshot and need refreshing after mutations. Changes to coordinated timing attributes must be applied atomically, because intermediate duration/key timing combinations can be invalid. Use the transaction tools rather than a series of independent updates. These are workflow constraints, not resolved MCP defects.
