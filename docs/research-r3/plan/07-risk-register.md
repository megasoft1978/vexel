# 07 — Risk register

Likelihood/impact: H/M/L. Owners are role placeholders, assign names at kickoff.

| ID | Risk | Likelihood | Impact | Mitigation | Trigger | Owner decision needed |
|---|---|---:|---:|---|---|---|
| R01 | Donner matches core wedge before launch | H | H | Build identical corpus benchmark and inspect round-trip now; differentiate only with measured gaps | Within 20% latency/RSS and same preservation | Continue, collaborate, or stop at M0/M10 |
| R02 | Inkscape 1.5 renderer improves target workflows | M | H | Compare stable + dev each gate; upstream contribution fallback | New build closes gap | Re-evaluate independent product thesis |
| R03 | SVG/CSS/filters/text compatibility balloons | H | H | Explicit support matrix, unknown-source preservation, limited MVP | >10% corpus silently differs | Widen, defer, or declare unsupported |
| R04 | Save corrupts metadata or changes rendering | M | Critical | Minimal-diff source spans, independent reparse/browser/Inkscape tests, backups | Any unedited subtree lost | Stop release until fixed |
| R05 | C memory safety defect on untrusted input | M | Critical | Limits, fuzz, sanitizers, audits, immutable snapshots | Crash, OOB, unbounded work | Security owner blocks release |
| R06 | Custom UI a11y/IME requires more time than core | H | H | semantic tree early, macOS manual access tests M6; budget native shims | Core workflow inaccessible | Add platform specialist / narrow UI |
| R07 | Stroke fidelity bottleneck | H | H | PlutoVG/vendor spike; early corpus; renderer adapter | >15% common stroke fixture failures | Vendor alternative or narrow support |
| R08 | Spatial index build too costly at 10M nodes | H | M | first paint before index, chunk/visible index, memory budget | index >5 sec or >500MB on target | limit editing count / defer full index |
| R09 | User demand is anecdotal, willingness to switch low | H | H | Interviews and pilots before M3/M10 investment | <5/15 pilot candidates | Stop or pivot to upstream work |
| R10 | C-only custom UI package not materially smaller | M | M | Measure entire bundle/dependencies, not LOC | >25MB package or no savings | Keep if UX wins; otherwise accept measured size |
| R11 | macOS native text/IME shape differs from Linux/Windows | M | H | service interface + platform conformance fixtures | layout differs after cross-platform open | Decide font/serialization policy |
| R12 | Thread shutdown/stale work causes use-after-free | M | Critical | generation handles, cancel/join order, TSan | stale render or sanitizer fault | Block merges/releases |
| R13 | Large embedded raster dominates “SVG” size | H | M | lazy retain, preview on demand, metrics separate bytes/geometry | base64 >input budget | offer safe inspect/skip, no eager decode |
| R14 | License/provenance contamination | L/M | H | clean-room notes, vendor manifest, contributor origin declarations | copied code/test/assets unclear | counsel review or remove component |
| R15 | Cross-platform platform maintenance overwhelms solo dev | H | H | Mac-first gate, recruit platform engineer before M11 | 2+ OS unreleased for 2 quarters | Narrow support commitments |
| R16 | No sustained funding for compatibility support | M | H | OSS sponsor/support plan and maintenance budget | <6 months funded runway before release | Reduce scope or seek sponsor |
| R17 | UI/render memory cache exceeds user budget | M | H | independent caps, LRU, counters, budget stress tests | RSS > target by >20% | Lower cache, alter indexing/layout |
| R18 | Name conflicts/trademark confusion | L | M | clearance before public beta; descriptive compatibility statement | conflict found | Rename before launch |
| R19 | Fuzz/corpus has licensing/privacy exposure | M | H | rights manifest, private corpus ACL, scrubbed uploads | unclear source or personal data | Exclude and replace fixture |
| R20 | Animation browser timing is inconsistent | H | M | SMIL/CSS subset, browser matrix, static fallback | output diverges across engines | Limit timeline/export subset |
