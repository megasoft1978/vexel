# 06 — Benchmark and quality system

## Benchmark protocol

1. Pin app commit/build flags, macOS version, machine model/chip/RAM/display scale/power mode, competitor version, corpus file SHA256 and warm/cold-cache state.
2. Close unrelated apps; run 5 cold and 10 warm trials; publish median, p95, min/max, standard deviation. If coefficient of variation >10%, investigate or mark noisy.
3. Measure separate steps: process launch; file read; parse/index; first visible pixel; all-visible render; steady pan/zoom; select; deselect; node drag; style edit; save; reopen in Inkscape/browser; peak RSS and allocation bytes.
4. Include 1/10/100MB, 500MB/1GB generated files with dimensions/semantic counts controlled independently: object-heavy, node-heavy, base64-heavy, filters, text, map, plot, generated tiny repeated shapes.
5. Compare Inkscape stable and current development build, Donner current release and current branch, plus browser renderer where relevant. State that this is product comparison, not isolated engine benchmark.
6. Keep results JSON/CSV and generated report in artifact storage tagged by machine/build. Never update baselines from a slower run without a review note.

## Proposed thresholds

- TTFP: 1MB simple <250ms; 10MB/100k simple elements <1s; 100MB map-like <2s; 1GB stress <5s for simple generated file or explicit safe cap/progress.
- UI responsiveness: no parse-induced event-loop stall >100ms; cached pan/zoom p95 <33ms; common 100k object selection/move p95 <50ms.
- Memory: report peak RSS; 100MB simple target ≤500MB peak; tile cache ≤256MB default; decoded embedded image cache ≤128MB default; process handles failures at configured budget. These are estimates/gates, not achieved results.
- Fidelity: 90% selected supported W3C/resvg static tests at M10; every skip labeled unsupported; 0 crash/security issues in corpus; 0 silent source drops.
- Source: unchanged no-op save byte-identical; edited save preserves all unedited source spans; undo/redo returns exact semantic state.
- Size: M0 measure; initial stretch stripped single-arch binary ≤10MB and app bundle ≤25MB; revise from measured system frameworks/fonts and vendor assets. Never hide debug symbols in distribution accounting.

## Test pyramid

- Unit: base arithmetic/slices/arena; XML lexical states; CSS/path parsing; transforms; tile blending; UI focus/layout.
- Property: transform inverse, path parse→serialize, attribute/source-span preservation, arena alignment, generation-handle invalidation, undo inverse invariants.
- Corpus/golden: manifest licensing, parser status, rendered PNG, visual tolerance, opened/saved reparse, browser/Inkscape consumer checks.
- Fuzz: XML, path data, CSS, transform, image headers, source serializer, use graph and filter graph. Run sanitizer-backed bounded-time jobs on PR and longer corpus/nightly.
- UI: headless paint bitmap and semantic tree snapshot; scripted pointer/key/IME; accessibility role/name/focus assertions; virtualized million-row model; native manual VoiceOver/IME tests.
- System: clean app launch, dialogs, clipboard, display scale, suspend/resume, file replacement, cancel and shutdown while jobs active.
- Performance: dedicated baselines separate from tests; stored artifacts and regression review.

## CI matrix and quality gates

PR minimum: macOS arm64 Clang debug+ASan/UBSan; Linux x86_64 Clang/GCC headless parser tests; Windows MSVC compile/core tests once M12. Nightly: fuzz 30 minutes per harness, TSan worker suite, corpus full render, x86_64 and arm64 performance if runners available. Release: notarization/signature, SBOM/license audit, clean install, package-size report, corpus run on all supported OS.

Performance regression policy: alert >10% stable p95 or >5% peak RSS; do not block on noisy shared runner without repeat. A 20% regression in a primary interaction requires owner and benchmark artifact. Store 12 months minimum (proposed); archive raw PNG diff and flame trace for release candidates.

## Apple Silicon CI

A self-hosted Mac mini is the preferred stable arm64 performance runner; Apple hardware hosted runners exist with provider-specific availability and rates that change. Exact procurement/cost is unknown until selecting GitHub Actions or another provider. Do not claim cloud runner cost in advance; record actual hourly/minute price and queue delay in ADR-010.
