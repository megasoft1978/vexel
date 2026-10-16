# Scientific evidence for Vexel rendering and responsiveness

Date: 2026-10-14. Work order: WO-RUST-RESEARCH-SCIENCE-01.
Requested skill: `science-paper-search`. This is a targeted literature review
for rendering, parsing resource bounds, and interaction latency within the
existing full Rust product goal. It does not establish performance gains or
complete the renderer, editor, native UI, or live MCP integration.

## Search and evidence

Used the free unauthenticated ScienceMadeReadable/OpenAlex route. The web tool
could not open its guide; the bundled standard-library helper fetched the
current guide successfully. All eight API requests returned HTTP200, including
five searches, one similarity expansion and one citation walk, plus the guide.
Requests were sequential and spaced beyond2.1seconds. No account, server
registration, credits, dependency, or private project information was submitted.

Queries searched GPU vector/path rasterization (semantic and exact terms),
bounded incremental XML, and input-feedback latency, then expanded the2016
scanline seed with similarity/citation queries. An exact inking-title search
checked publication versions. Raw timestamped request/response envelopes,
`query-log.json`, `screened-records.json`, `version-reconciliation.json` and
hashed primary PDFs/text extractions are in the work-order artifacts.

The index covers titles/abstracts, not full papers. This is not an exhaustive
survey or a ranking-based algorithm decision. Some exact-word results concerned
vectorization, neural generation or3D rendering and were excluded as different
tasks. Citation walking suggested silhouette texture magnification; that is
outside the present SVG editing scope. No external implementation was copied,
no GPL editor source was read, and code licenses were not established: any
future adoption needs separate provenance/dependency review.

## Relevant primary evidence

Full texts were obtained for the first five entries below; methods,
experimental setup, evaluation and limitations relevant to Vexel were examined,
rather than claiming every section was reviewed. The final entry remains a
retrieved abstract shortlist because its author endpoint failed TLS hostname
verification; certificate checks were retained.

| Paper | What the evidence supports | Limits for Vexel |
|---|---|---|
| [Nehab & Hoppe, Random-access rendering of general vector graphics (2008)](https://hhoppe.com/ravg.pdf) | Cell-local descriptions reduce work to spatially relevant primitives; extended cell regions support filtering. The paper explicitly discusses static-layout/re-encoding and unsupported-attribute limitations. | DirectX10/GeForce8800GTX, not AppleMetal. Include preprocessing, replicated storage, edit invalidation, thin strokes and corner quality in comparisons. |
| [Ganacim et al., Massively-parallel vector graphics (2014)](https://w3.impa.br/~diego/publications/GanEtAl14.pdf) | A shortcut-tree pipeline parallelizes preprocessing by segment and rendering by sample/pixel. Evaluations use a GTXTitan and report preprocessing separately from rendering. | Old GPU/hardware and selected vector illustrations. Raster timings alone do not establish editor latency, SVG fidelity or whole-frame rebuild cost. |
| [Konrad & Magniez, Validating XML Documents in the Streaming Model with External Memory (arXiv1012.3311)](https://arxiv.org/abs/1012.3311) | General DTD validation has space/pass trade-offs; the external-memory algorithm uses additional streams and passes. | The downloaded revision is2021; initial preprint2010, with later conference/journal versions. This is DTD validation, not our lexical source-preserving parser or editable DOM. It does not prove constant-memory editing. |
| [Ng et al., Designing for low-latency direct-touch input (UIST2012)](https://www.tactuallabs.com/papers/designingLowLatencyDirectTouchInputUIST12.pdf) | Ten right-handed participants compared direct-touch dragging delays on special low-latency hardware. The paper proposes immediate simple feedback followed by detailed visuals. | The experiment measures perceptual discrimination during touch dragging. It does not establish Vexel or general application performance, or macOS mouse/tablet productivity. Low-millisecond observations are not universal acceptance thresholds. |
| [Annett et al., How Low Should We Go? (Graphics Interface2014)](https://webdocs.cs.ualberta.ca/~wfb/publications/C-2014-GI-Latency.pdf) | Writing/sketching with a custom stylus system produced different perceptual thresholds from simpler dragging. The first experiment uses12 participants and examines just-noticeable differences. | Perception differs from tolerance and task completion. The indexed2020 chapter is treated as a related version, not another replication. No blanket50ms product threshold follows. |
| [Li, Hou & Zhou, Efficient GPU path rendering using scanline rasterization (2016)](https://doi.org/10.1145/2980179.2982434) | Retrieved abstract describes boundary-fragment work and bulk interior spans. This is an abstract-only lead for later primary-source review, with no recommendation. | Full author PDF unavailable in this run. Reported speedups are not verified here or transferable to AppleSilicon. |

## Testable implementation directions

These are engineering inferences from the evidence, not measured Vexel results.

1. **Retain spatially local rendering and bounded caching.** Keep source/vector
   data editable and preserved. Evaluate coarse tile/cell culling and local
   invalidation; measure overlap amplification and cache metadata as well as
   pixels. Compare against the owned CPU renderer before choosing a Metal path.
2. **Include every stage in animation costs.** Measure decoding, path expansion,
   spatial preprocessing, invalidation, rendering and presentation separately
   and together. Use both static pan/zoom and changing artwork/timeline input;
   caching a static illustration cannot establish animated-frame performance.
3. **Separate progress from validation.** A partial preview must not certify an
   unvalidated document. Keep bounded per-step work, cancellation and explicit
   storage errors; charge actual reserved token/table capacity. Streaming
   visitors and retained editable models have different memory requirements.
4. **Measure feedback by task and input.** Record input-to-first-visible-feedback
   and input-to-final-frame for drawing, dragging/nodes, pan/zoom, playback and
   live MCP transactions. Fast previews must converge to faithful output and
   visibly report transaction/progress/cancel state, preserving undo integrity.
5. **Preserve existing product budgets.** ADR0018’s100MB/2second first-content,
   <100ms owner-thread pause and p95<33ms pan/zoom targets remain unchanged.
   Later work orders must name AppleSilicon hardware/OS/build, corpus hashes,
   cold/warm protocol, repeated p50/p95/worst samples and memory/quality results.
   Record end-to-end latency separately from renderer duration. No new broad
   UI/performance suite ran during this research.

## Gaps and next evidence

AppleMetal equivalence, full SVG clips/masks/text/effects, dynamic source editing,
MCP transaction observability and representative-user measurements remain open.
The review does not select an external renderer or retire the owned parser.
Native animation authoring and Rust/AppKit integration still follow the full
product plan. Comprehensive UI/E2E, fidelity, performance, accessibility,
recovery, packaged-release and authentic recorded demos remain final gates.
