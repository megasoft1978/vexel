# LLM-aware design and verification changes

## Code structure for context-sized work

Keep C17 and the project’s existing architecture. Design boundaries around reviewable invariants: base/slices/arena; bounded XML tokenizer/source spans; path numeric lexer/parser; geometry; style; scene; raster; spatial index; document overlays/serialization; editor transactions; UI core/widgets; platform shims. Each module gets a short README, public header, ownership/threading/error contract, limits, and focused test target. No “god” files.

Targets (guidance, checked by a script but exceptions documented): public header <=300 lines; implementation translation unit <=800 lines; function <=80 logical lines; public functions <=12/module; aim for 100–300 line files. Split by invariant, not arbitrary line count. Explicit names (`svg_path_parse_number`, `doc_overlay_set_attribute`), no opaque abbreviations, no clever macro metaprogramming. Macros only for simple constants/compiler attributes/compile-time checks. Avoid hidden global state, complicated callback ownership, implicit conversions, and APIs with ambiguous buffer capacity.

C remains locked. C’s predictable ABI and low runtime dependency footprint fit the product, but AI-written C raises the cost of memory-safety review. Require explicit `bytes {ptr,len}`, checked add/mul, capacity-aware buffers, arena ownership, generation handles, single-owner mutation, thread snapshots, error returns, and comments on lifetimes. Use Clang `-Wall -Wextra -Wpedantic -Wconversion -Wshadow -Wstrict-prototypes -Wmissing-prototypes -Werror` as staged profiles (some noisy conversions isolated); debug ASan+UBSan by default, fuzz builds with sanitizers; MSan where supported; static analyzer; consider Clang `-fbounds-safety` experimentally only after target compiler support and ABI review. Never claim it substitutes for tests. Build warning policy can exempt specific documented diagnostics, not blanket-disable.

Deterministic build: pinned tool versions, generated files isolated, no configure-time downloads, fixed locale/timezone, explicit floating-point behavior where goldens require it, source and dependency hashes, reproducible archive/bundle metadata where feasible. One-command developer entry: `./dev build`, `./dev test-fast`, `./dev verify`, `./dev bench`; inner focused tests under 30 seconds. Heavy full corpus/fuzz/perf suites are separate jobs. Benchmark instability is reported, never “fixed” by loosening thresholds.

## Strengths and weak spots: process compensation

| LLM weak spot | Concrete compensation |
|---|---|
| Numerically robust geometry (stroke joins/caps, transforms, booleans) | Prefer mature pinned algorithms behind adapters; scalar reference implementation; property tests and degenerate-input generators; differential render against resvg/Skia/Inkscape; invariant tests; exact issue corpus. Defer booleans. Any boolean engine gets its own acceptance gate. |
| SIMD/performance tuning | Start scalar, establish profile, only vectorize measured kernels; scalar/SIMD differential tests; benchmark on fixed corpus and exact hardware; no unmeasured “optimization.” Keep portable fallback. |
| C lifetime/bounds bugs | Safer slice APIs, checked arithmetic, arenas, fuzzing, sanitizers, analyzer, review memory ownership paths; crash repro fixtures and symbolized reports. Protected tests cannot be changed within feature WO. |
| Cocoa/Metal/Wayland/IME/API quirks | Compile/run native smoke tests, official versioned API docs in WO, narrow platform shims, nightly SDK CI, feature probes, API prototype WO before UI layer depends on it. Real-human gates for IME/accessibility/compositors. |
| Visual/UX judgment | Deterministic screenshots and golden diffs catch regressions, Luna image-input review catches obvious clipping/contrast/layout errors, but owner reviews named workflows at M3 and release. Screenshot model is not product authority. |
| Flaky timing/perf | Separate performance job, warm-up and repeated trials, machine metadata/thermal notes, median + confidence interval, fixed baseline and effect threshold, no pass/fail from a single run. |
| Reward hacking | Frozen independent oracle and corpus hashes; tests excluded from allowed paths; test mutation audit; differential references; hidden holdout fixtures owner controls; run mutants/negative controls; compare claim to raw artifact. A hidden test is not enough if model can infer it. |
| Long-horizon coherence | One WO at a time, state/ADR/README, contract tests, fresh-session verifier, milestone architecture audit, no huge multi-feature tasks. |
| Hallucinated APIs | Codex must use installed SDK headers and official docs, compile minimal API probe first, report exact doc URL and tested SDK. Verifier independently checks symbols/build. |
| License contamination | Executor cannot read Inkscape source in normal tasks; benchmark comparison uses binaries and black-box fixtures only. Record provenance of borrowed algorithms, dependency SBOM/SPDX, license scanner, similarity scan on suspicious copied code, legal review before distribution. No code generated from copyleft snippets in prompt. |

LLMs are strong at breadth, boilerplate, test/fuzzer harness creation, mechanical refactors, porting well-specified known algorithms, documentation, and exploring error cases. The harness should assign these freely while putting semantics and pass/fail evidence behind external oracles.

## Own vs vendor under AI economics

AI code is cheap; verification, long-tail behavior, and maintenance are not. Minimize implementations of standards with many edge cases. Keep dependencies behind stable adapters and license/pin them. “Minimal dependencies” means deliberate, audited set—not writing every library ourselves.

| Component | Decision under this model | Why / boundary |
|---|---|---|
| XML parse | Vendor a small, permissively licensed, fuzzed tokenizer/parser if it meets source-span/preservation needs; otherwise write a narrow byte tokenizer, not a validating general XML stack. Evaluate Expat/libxml2 after product/license/size review. | Entity expansion/security and XML corner cases are costly. Preserve original bytes separately; no parser should rewrite on open. |
| SVG scene/render | Vendor PlutoVG/PlutoSVG or another permissive backend behind adapter as an early baseline; compare coverage, C API, build size/license. Do not write a complete rasterizer before measured need. Keep a tiny scalar renderer for test oracle/basic primitives. | Raster/stroke correctness is major verification sink; backend may not cover advanced SVG, but can prove UI/workbench loop sooner. |
| Path geometry | Own bounded SVG path-data lexer and normalized command representation; vendor robust flatten/stroke primitive if benchmark and API fit. Own the editor-specific node model. | Path grammar/round trip is small and source-preserved; robust stroke expansion is a high-risk independent algorithm. |
| CSS/style | Use a small documented subset initially; vendor CSS parser if permissive and dependency cost acceptable; own SVG cascade subset mapping. | CSS parsing mature; SVG cascade semantics still product-specific. Unsupported rules remain source-preserved. |
| Text shaping/fonts | Vendor/use system CoreText on Mac; HarfBuzz + FreeType for portable targets (explicit deps). | Writing shaping is not defensible; exact platform output differs and needs fallback tests. |
| Image decode | Vendor established small decoders with current security updates (e.g., stb_image only after license/version/audit), or system APIs if portable. | Malformed images are attack surface; fuzz and pin regardless. |
| JSON work-order/driver tooling | Python stdlib scripts, not app dependency. | Operational tooling can prioritize clarity. |
| UI framework | Custom-drawn UI remains locked. Use OS APIs only for windows, events, text, dialogs, accessibility, and presentation. | Product decision; compensate with UI driver and human checkpoints. |

Re-evaluate each vendored library at M0 with exact source size, transitive deps, license, build time, symbols, corpus fidelity, fuzz history, CVE process, and update owner. AI generation does not make “own” the default; it makes readable adapters and verification artifacts essential.

## Verification without continuous human eyes

- Headless renderer outputs PNG at deterministic frame/time/scale; golden comparison uses exact pixels for scalar deterministic fixtures and documented perceptual/edge tolerance for antialiasing differences. Store per-pixel heatmaps, masks, expected hash, generator version, SVG hash, renderer version. Golden changes require separate WO and fresh reference render.
- `svgworkbench --script events.jsonl --open file.svg --frames 30 --dump-ui ui.json --screenshot out.png --exit` is a first-week driver. JSON commands: open, wait-for-index-progress, viewport, pointer, key, text-commit, accessibility query, screenshot, assert, save, quit. Virtual clock/frame stepping and seeded worker scheduling make replay deterministic. UI must not expose unsafe arbitrary scripting to SVG content.
- Accessibility tree snapshot assertions test roles/names/states/focus order/keyboard operation; native adapter snapshots checked on OS. This does not prove real VoiceOver or screen-reader usability.
- Record sanitized event streams for failures; never record typed secrets. Replay with app version and input SHA.
- Luna may inspect screenshots by `codex exec -i`; save its structured findings and image hash. Its visual judgment flags obvious defects only. Owner reviews M3 sample screens and beta workflows.
- Run parser/property fuzzers with corpus rotation, fixed seeds for regression, time/memory limits, crash dedup by sanitizer stack, minimized reproductions, and batch symbolication (`lldb -b -o 'bt all' -o quit -- <binary>` or OS crash-report tooling). Each crash artifact stores build UUID, binary/dSYM hash, input hash, signal, sanitizer stack and reproduction command.
- Performance: use one dedicated Apple Silicon Mac; close competing workloads; record chip, RAM, macOS, power source/Low Power Mode, available thermal telemetry, build SHA and corpus SHA; 5 warmups + 15 measured trials; compare medians, p95 and bootstrap interval. macOS does not offer a dependable portable CPU-core pinning method for this harness, so do not claim affinity pinning: control concurrency, use the same power/thermal conditions, and defer if throttled. Proposed regression gate requires >10% slowdown with interval outside noise on two runs. Linux/Windows CI can validate compile/tests but not claim Mac render timing.

### Cross-platform reality on one Apple Silicon Mac

GitHub Actions currently offers Linux x64/ARM64, Windows x64/ARM64, and macOS hosted runners; standard hosted Linux and Windows are the inexpensive compile/test matrix, while standard macOS minutes cost substantially more. Hosted macOS runners are useful for SDK compile/smoke checks but do not match the owner's hardware and are not a performance oracle. Verify runner availability and pricing when enabling CI.

UTM can run Linux and Windows-on-Arm guests locally. Linux containers share the host kernel and are excellent for parser/build tests, but do not reproduce graphics drivers, desktop sessions, or compositor behavior. For headless Linux UI, run a real Wayland compositor such as Weston with its headless backend or Cage under a virtual display; Xvfb covers X11-only paths and does not prove Wayland support. Use scripted input and PNG capture within that compositor, then retain one real compositor owner check before a support claim.

Windows on an Apple Silicon Mac means Windows on Arm virtualization; x64 emulation does not reproduce x64 native GPU, driver, IME, or accessibility behavior. Wine is useful for basic binary smoke tests but cannot establish native Windows support. Use GitHub's Windows x64/ARM64 runners for compilation and scripted tests where available, then owner-selected real Windows hardware for release support evidence. Keep Mac-first release scope until native Windows checks are available.

## Human gates that automation cannot replace

1. M0 30-minute owner review of corpus realism/interview result and competing workflow comparison.
2. M3 45-minute screen review + opening/editing/saving one realistic document on the target Mac.
3. Pre-beta one pass of real VoiceOver, real CJK composition/IME, app feel/keyboard discoverability; owner records concise checklist.
4. Before support claims, brief real Linux Wayland compositor and Windows hardware/IME spot check if a suitable machine is available. Until then label experimental.
5. Animation only after an owner demand decision.

These are explicit decision gates, not a request for a human engineer to implement features.
