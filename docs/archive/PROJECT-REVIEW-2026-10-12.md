# Vexel project review — 2026-10-12

Owner-requested deep review of the target, the delivered work, the planned work,
and the open risks. No product code, test, golden, or oracle was changed. The
only repository changes made alongside this review are verification records and
ledger closure for two already-frozen work orders.

Method: read the governing documents (`OWNER-DIRECTIVES.md`, `AGENTS.md`,
`docs/research-r1..r4`, `docs/adr/*`, `work/WO-*.md`, `STATE.md`, `LEDGER.md`,
`ledger.jsonl`, `BLOCKERS.md`, `HANDOFF.md`, `docs/lanes/*`), audited the tree
against the roadmap, re-ran the project's own gates, and reproduced the
real-corpus behaviour directly with the built Debug and Release binaries.

---

## 1. What the project is trying to be

The governing vision is the round-3 product scope, carried forward by the
round-4 roadmap:

> "Open, inspect and lightly edit huge SVGs without waiting for the entire
> document or losing source data." Initial users: GIS/cartographers,
> scientific-visualization authors, and selected laser/plotter workflows.
> Position as a large-SVG workbench, not a parity replacement.
> — `docs/research-r3/plan/01-product-scope.md`

Locked decisions, all still in force: Apple Silicon macOS 13+ only, C17,
MPL-2.0, minimal dependencies, custom-drawn UI, an owned SVG parser and
rasterizer, no GPL/Inkscape source, no network fetching of SVG resources
(`OWNER-DIRECTIVES.md`, `docs/adr/0001`, `docs/adr/0002`).

Declared pass conditions for the proof phase (`docs/research-r4/00-verdict-and-changes.md`):
first visible pixel under 2 s for a 100 MB map-like SVG on named Apple Silicon;
no main-thread pause over 100 ms during indexing; visible pan/zoom p95 under
33 ms; no-op save preserves source bytes; **at least five target users provide
representative files/tasks and agree to pilot**; no known critical data-loss or
memory-safety issue.

Declared human gates: M0 (user/corpus signal, continue/stop), M3 (review demo +
pilots), before beta (workflow feel, VoiceOver, CJK IME, sign/notarize), before
platform claims, before animation (separate demand and scope gate), and release.

## 2. What has actually been built

The tree is far past the roadmap's own frontier in breadth. Audited against
`docs/research-r4/05-revised-roadmap.md`:

- 302 C/Objective-C sources under `src/` + `app/`; **80 registered CTest
  targets**; 41 Python test modules; no source file over 800 lines (largest
  799).
- **Implemented and tested:** geometry, path grammar, fills/antialiasing,
  strokes, a bounded CSS cascade, gradients/clip/`<use>` (M4-01..06); stable
  document handles, piece-table overlays, minimal-diff save (M5-01..03);
  retained UI tree, layout/paint, focus/keyboard/semantics, CoreText text
  adapter and an NSAccessibility bridge (M6-01..03); hit testing, selection
  overlays, transforms (M7-01..03); style controls, layers/order, path-node
  editing (M8-01..03); transactional undo/redo with bounded history, atomic
  save/save-as (M9-01..02); SMIL evaluation, keyframes, easing, deterministic
  animation frames and full keyframe authoring (part of M13).
- **Off-roadmap but shipped:** a local **MCP server** (`src/mcp/**`,
  `docs/mcp.md`), platform media providers (image/GIF/CoreText), a first-run
  Welcome screen with recents and multi-document tabs, a ⌘K command palette,
  rulers/guides/snapping, PNG export, clipboard, and localization.
- **Absent:** renderer bake-off (M4-07), fidelity gate (M4-08), recovery
  journal (M5-04), crash recovery (M9-03), the 100k-row inspector gate
  (M6-04), the discoverability gate (M8-04), package/SBOM proof (M10-03), the
  pilot gate (M10-04), the animation demand ADR, and CSS-animation timelines.

## 3. Verified health of `main` at `248fbfa` (today's runs)

| Gate | Result |
|---|---|
| `./dev verify` | CTest 80/80; Python 302/302; native smoke + 20-frame Metal replay |
| `./dev asan` | CTest 80/80, no sanitizer report |
| `./dev ubsan` | CTest 80/80 |
| `./dev t --all` | CTest 80/80; Python 69/69 |
| Debug/Release configure+build | exit 0 |
| `tests.test_platform_driver` | 13 tests pass |
| 100-cycle native platform smoke | pass, 3.13 s |
| `ledger.jsonl` | valid, 459 entries |
| `wo_validate` (both open work orders) | valid |
| `git status --porcelain` | clean before the closure work |

The code base is genuinely green. Every problem below is about product outcome,
evidence, or governance — not about a broken build.

## 4. Findings

### F1 — The headline corpus metrics do not measure rendering the document

`reports/M4/head-to-head.md` reports that of 58 real corpus files (4.43 GB),
2 render ("ok") and 56 are "unsupported", and derives 41.6× first-preview speed
and 30.4× peak-RSS ratios from those 2 files.

I reproduced the pipeline and found what "first preview" actually is.
`svg_index_make_preview_source` (`src/svg/index/index.c:419-440`) **synthesizes a
new document**: the root's opening tag, then exactly one shape element taken
from the index, then `</svg>`, capped at 16 MiB. `app/index_bench.c` renders that
synthetic document. So the benchmark's `first_preview_ns` is "time to render one
extracted element", not the time to first paint the file.

Concretely, on the same two "ok" files:

```
$ build/mac-release/bin/svg_index_bench .../real/world-map-blank.svg
{"outcome":"ok",...,"status_code":6,"diagnostic_category":0,"element_count":248}
$ build/mac-debug/bin/svgworkbench --render-frame .../real/world-map-blank.svg --size 512x512 --out /tmp/x.ppm
render-frame: world-map-blank.svg: unsupported at byte 828     (exit 1)
```

The same holds for `world-map-countries.svg`. So the two reported successes are
successes of the synthetic-preview path; neither file renders as a document.

I then measured the real thing — full-document render through
`svgworkbench --render-frame` over every file in the pinned 58-file manifest that
is small enough to probe (39 of 58; the other 19 exceed 30 MB and were skipped
by this sweep):

| Result | Count |
|---|---:|
| Rendered (`exit 0`) | **1** (`files/generated/embedded_t1.svg`, 912 KB) |
| `limit` | 26 |
| `unsupported` | 8 |
| `xml` | 4 |

**Of the 39 pinned corpus files measured through a real render, 1 succeeds.**
The 41.6x/30.4x figures compare a single-element synthetic preview against
Inkscape rendering whole documents; the report's own "protocols differ" caveat
understates a material difference in what is being measured. The 100 MB "first
visible pixel < 2 s" gate is not measured by this pipeline at all, and the
projection/rasterizer performance evidence in `docs/PERF.md` rests on a preview
that does not correspond to a full render.

A note on corpus drift: the manifest pins 58 files, but 81 `.svg` files are
present on disk at the corpus root (23 under `files/real_inkscape/` are not in
the manifest). The manifest hash still matches the value recorded in the
reports, so the reports describe the pinned 58; the extra files are
unmanifested.

### F2 — The corpus report cannot say why any file fails

`index_bench` already emits `status_code` and `diagnostic_category` per file,
but `reports/M4/head_to_head.py` discards them, so
`reports/M4/head-to-head.json` records only `ok`/`unsupported`. The single most
valuable measurement for the renderer programme — which feature or limit blocks
which file — is available and thrown away. `reports/M4` has no fidelity
coverage matrix.

### F3 — I reproduced the blockers; most are policy, not missing features

Minimal counterexample documents probed with `--render-frame`:

| Cause | Reproducer | Corpus reach |
|---|---|---|
| **Unknown non-namespaced attributes are fatal** | `<path foo="0" d="M0 0L1 1"/>` → `unsupported`; `<path id="a-b" .../>` and `<path class="land" .../>` → OK. `svg_reference_attributes_supported` (`src/svg/reference/reference.c:222-315`) fails closed on any attribute outside its whitelist, while *namespaced* unknowns are skipped (`:289-296`) | `generated/map_t1.svg` dies at its first `data-region` path (2,512 `data-*` attributes in that file); `real/coat-of-arms-of-villa-alemana-svg.svg` and `real/comunas-occitanas-d-alemanha-svg.svg` also carry them |
| **`stroke="url(#gradient)"` unsupported** | `<rect ... fill="none" stroke="url(#g)"/>` → `unsupported`; identical `fill="url(#g)"` → OK | `generated/inkscape_t1.svg` dies at byte 1214 on exactly that value; `@stroke` appears in 25 files and gradients in 38 |
| **`<meta>` element unsupported** | `<meta name="x" content="y"/>` → `unsupported` | `<metadata>` is skipped but `<meta>` is fatal — asymmetric metadata policy |
| **Document-wide glyph cap** | `SVG_SHAPE_MEDIA_MAX_GLYPHS` (16384) at `src/svg/shape/text.c:568` | `generated/text_t1.svg` dies inside a `<text>` run at byte 15500; `cad`, `cnc`, `text`, `use` families |
| **Path command cap** | `generated/giant_path_t1.svg` dies inside a path `d` at byte 603597 | `giant_path` family |
| **XML nesting cap reported as an XML error** | `generated/nested_t1.svg` reports `xml at byte 16843` at a `<g>` boundary in a well-formed file | `nested` family |
| **Unlocatable limits** | `generated/science_t1.svg` and `generated/embedded_t2.svg` report `limit at byte 0` | `science`, `embedded` families |

The dominant failure category is `limit` (44/62), i.e. deliberate caps that
refuse whole documents instead of degrading. The MVP scope promises the
opposite: unsupported-feature diagnostics, preservation of unknown content, and
"0 silent unsupported drops" (`docs/research-r3/plan/01-product-scope.md`).

### F4 — Governance: most of the shipped product has no work order

Only 45 of 432 commits (10.4%) name a work order. Thirty-four lane merges
touched 676 paths — **240 of 310 `src/` files (77%) and 345 of 509 `tests/`
files (68%)** — and none maps to a frozen `work/WO-*.md`. The W8–W16 lanes
(rulers, palette, selection, node editing, timeline, keyframes, visual scale,
native shell, accessibility, plus the MCP server and media providers) were
merged into `main` with no ledger lifecycle event; several lane docs say so
explicitly. `work/WO-M6-01.md` and `work/WO-M6-02.md` have no front matter at
all, so `scripts/wo_validate.py` rejects them and `ledger.jsonl` can never
record them; their governance lives in a parallel `STATE-laneB.md` /
`LEDGER-laneB.md`.

The spec-hash control is sound: all 39 frozen orders' `spec_sha256` values
recompute correctly. But the validators check only front-matter hashes and
ledger transition legality. Path scope is enforced only at run time by
`scripts/wo_driver.py`; "one commit per work order" is enforced nowhere, and
`base_sha` is never checked against the git graph.

### F5 — Two work orders were left open, one behind a real defect

- **WO-M1-01 revision 3** stopped at `./dev asan`: CTest 77
  `animation-authoring-edge` aborted with an AddressSanitizer
  heap-use-after-free in `authoring.c:113`, reached through
  `animation_authoring_edge_test.c:468`, because the test freed the borrowed
  animation source before `svg_anim_authoring_add_key`. Its verifier run had no
  verdict.
- **WO-ANIM-FIX-01** was frozen to fix exactly that; the fix was committed
  (`c95ad83`) and the order was never closed. Its recorded verdict was `fail` —
  and that reason was itself wrong, because the committed evidence already
  contained a passing `./dev t --all` record.

Both are now closed; see section 6.

### F6 — Status documents are self-contradictory and stale

- `BLOCKERS.md:3` says WO-M4-06 is closed and verified; `BLOCKERS.md:5-17` still
  presents it as blocked with "a fresh verifier has not run".
- `HANDOFF.md` is titled "WO-M4-06 handoff — round 7", says "The work remains
  uncommitted", and names neither open work order; its own last line says
  "Superseded".
- `STATE.md` is an append-only 325-line diary, although the operating model
  requires a current summary. It contains three different "active work order"
  statements (`:77` M2-01, `:138` M4-06, `:321` "M4-06 merged; next M4-07") and
  never mentioned WO-M1-01 r3 or WO-ANIM-FIX-01. It still reports the ledger as
  "412 valid entries"; the file has 459.
- `README.md` documents a viewer with fit/zoom/pan/resize/quit and a one-row
  status table, while `main` ships an editor with timeline/keyframe authoring,
  node editing, tabs, accessibility/localization and an MCP server.
- `LEDGER.md` is a second, hand-maintained ledger with no row for the two open
  orders.

### F7 — Repository hygiene and a disconnected lineage

- `git worktree list` reports 98 worktrees; `.worktrees/` is **21 GB**
  (`WO-M4-06` alone 9.1 GB). `build/` is 792 MB, `.cache/` 50 MB.
- 15 branches are unmerged, and several (`integrate/final-ui-docs`,
  `integrate/vexel-main-migration`, `worktree-agent-ac9b3f7832e7ddfc2`,
  `plan/ci-nightly`, `plan/ui-golden-audit`, `wo/WO-UI-DOCS-*`,
  `fix/ui-polish-main`) share **no merge base with `main`**: they descend from a
  second root, `d97da2f` "Initial public snapshot" (2026-10-09). That lineage
  holds ~36 commits of UI-golden, CI, and docs work that the owner may believe
  has landed and has not.

### F8 — Owner gates are all still open, and one was bypassed

| Gate | State |
|---|---|
| M0 user/corpus signal | OPEN (`reports/M0/product-evidence.md`); 0 user files, 0 interviews |
| M3 review demo + pilots | OPEN (`reports/M3/decision.md`); 0 pilots, 0 outcomes |
| M3→M4 proceed | Partially closed by `docs/adr/0012`, which explicitly leaves the M4+ scope gate open |
| M4 fidelity scope (M4-08) | OPEN, no record; no `work/WO-M4-07/08` exists |
| M6 visual/interaction | Not closed; every lane defers "maintainer real-window review" |
| M8 discoverability | No record |
| M10 ship/pivot/stop | Not reached |
| M13 animation demand gate | **Bypassed** — animation shipped via lanes and WO-MOTION-03/04 with no demand/semantics ADR |
| M11/M12 | Void per `OWNER-DIRECTIVES.md` |

### F9 — Risk controls that are documented but not implemented

Missing from the tree although the risk register prescribes them:
`bench/corpus/manifest.lock` (referenced as frozen by `wo_driver.py:34` but
non-existent — the corpus uses `manifest.json` + `manifest.sha256`), any
mutation audit, hidden owner-held fixtures, thermal metadata in
`scripts/perf_gate.py`, SBOM, similarity/CVE/license scanning, quota metering,
and the nightly stale-STATE / orphan-worktree / weakened-assertion audit
(`.github/workflows/nightly.yml` runs only e2e, asan, ubsan, soak). No hosted CI
run has ever been observed. `./dev tsan` covers a single test target.

## 5. Planned versus delivered

Roadmap orders worked: BOOT-01..08, M0-01..04, M1-01..04, M2-01..05, M3-01..06,
M4-01..06, M5-01, plus WO-UI-GOLDEN-01, WO-MOTION-03/04, WO-ANIM-FIX-01. All are
merged, the last two as of this review.

Roadmap orders **not** worked, and now out of sequence with reality: M4-07,
M4-08, M5-02..04, M6-03/04, M7-*, M8-*, M9-*, M10-*, and M13's demand ADR —
even though M6–M9 and most of M13 are already implemented in code.

## 6. Actions taken in this review

1. Diagnosed the real corpus blockers and measured the true full-render success
   rate (1/62 probed) — the first time this has been quantified.
2. Re-ran and recorded every frozen command for **WO-ANIM-FIX-01** and
   **WO-M1-01 revision 3**; all commands exit 0. Evidence:
   `artifacts/WO-ANIM-FIX-01/r1-verify/` and `artifacts/WO-M1-01/r3-verify/`.
3. Ran fresh independent read-only verifiers for each order, in rounds. Round 1
   found four real evidence defects (a trailing blank line that made
   `git diff --check` fail, a missing AC-7 verdict artifact, and a benchmark
   record that contradicted the built binary). All were corrected with the
   round-1 failures retained rather than erased. Round 2 affirmatively
   established every substantive criterion — including that both sanitizer runs
   were genuinely instrumented and that the fix frees the borrowed source
   exactly once after document destruction without weakening an assertion — and
   objected only to the ordering of one recorded command against a later
   evidence write, and to the verdict artifact that the verification itself
   produces. A final narrow round confirmed the closed state.
4. Refreshed `bench/results/WO-M1-01.json` with the true current Release size
   and test counts, keeping the revision-1 measurement explicitly as history.
5. Closed both orders in `ledger.jsonl` and updated `STATE.md` / `LEDGER.md`.

## 7. Recommended next steps (owner decision required)

1. **Make the real corpus render, and measure the right thing.** Replace
   fail-closed attribute handling with ignore-and-diagnose; support gradient
   strokes; make document caps degrade rather than refuse; and either render the
   whole document for `first_preview_ns` or rename the metric to what it is.
   Until then the wedge has no evidence and every fidelity gate is
   unmeasurable.
2. **Record per-file diagnostics** in the corpus report so the renderer
   programme can be ordered by real impact instead of by element counts.
3. **Re-baseline the plan against reality** — freeze work orders for what is
   already shipped or formally grandfather it, then sequence M4-07/M4-08
   against the corpus work rather than before it.
4. **Reconcile the status surface** — rewrite `STATE.md` as a summary; fix
   `README.md`, `BLOCKERS.md`, `HANDOFF.md`; decide the fate of the second-root
   lineage and the 21 GB of worktrees.
5. **Decide the fidelity scope** (M4-08): text, images, filters, patterns,
   masks, markers, CSS animation — what is in the first release.
6. **Get user evidence**, or formally accept that M0/M3 proceed on synthetic +
   public-corpus evidence only, as `ADR-0012` allowed.
