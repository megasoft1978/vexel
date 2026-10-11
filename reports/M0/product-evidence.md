# M0 product evidence packet

**Evidence date:** 2026-10-08
**Status:** Repository evidence recorded; owner-provided product evidence absent; owner decision open.

This packet separates observed repository records from missing owner/user signal. It makes no claim about customer demand or market fit. Machine-readable counts, results, availability reasons, and source SHA-256 values are in [evidence.json](evidence.json); run `python3 reports/M0/validate_packet.py` to recheck them.

## Recorded user/product signal

The cited repository evidence contains **0 owner-provided files or task records, 0 recorded interviews, and 0 user outcome records**. No owner responses are available yet. Existing tests, harness output, and project-authored fixtures demonstrate engineering activity only; they do not demonstrate user demand, adoption, task success, or customer outcomes. The corpus provenance identifies its 24 files as synthetic, project-authored examples ([docs/corpus/PROVENANCE.md](../../docs/corpus/PROVENANCE.md)).

## Corpus and provenance

The pinned corpus has 24 project-authored MPL-2.0 synthetic fixtures, and the golden lock has 24 pinned fixtures. Four corpus fixtures have the `stress` feature tag; these small feature examples are not benchmark-size tiers. The corpus manifest and its digest are [bench/corpus/manifest.json](../../bench/corpus/manifest.json) and [bench/corpus/manifest.sha256](../../bench/corpus/manifest.sha256); pinned golden inventory: [tests/golden/manifest.lock](../../tests/golden/manifest.lock). Provenance explicitly excludes natural/customer files and cautions that this subset establishes neither conformance nor representative performance.

## Competitor baseline

M0-03 checked two competitors. **Inkscape and Donner are unavailable**: neither executable was found, so no competitor render comparison exists. M0-03 ran in smoke mode and recorded no trials ([bench/results/WO-M0-03.json](../../bench/results/WO-M0-03.json), [bench/results/WO-M0-03.csv](../../bench/results/WO-M0-03.csv)). Vexel also has no compatible SVG render command in that result; a version/startup probe is not an SVG render benchmark.

## Measured results

M0-02 generated exact 10,000,000-byte object-heavy, 100,000,000-byte path-heavy, and 1,000,000,000-byte mixed documents in 0.010086 s, 0.086280 s, and 0.587311 s respectively. The mixed document SHA-256 was `895780c50a7e3b6f05e1ad34b68ca6d22684699e6fa1b7425a7f1baaa6caa9e7`. These are **input-generation timings**, not Vexel render timings. M0-02 also re-rendered the 24 pinned 256×256 oracle fixtures with librsvg 2.62.3 in 0.700504 s; this is oracle activity, not a Vexel performance result ([bench/results/WO-M0-02.json](../../bench/results/WO-M0-02.json), [artifacts/WO-M0-02/oracle-render.txt](../../artifacts/WO-M0-02/oracle-render.txt)).

M0-03 has **0 measured adapter trials**. It records **0 RSS measurements**: the bounded standard-library process interface does not provide peak RSS. There are no Vexel-versus-competitor render results, no RSS comparison, and no compatible Vexel render measurements. See [bench/results/WO-M0-03.json](../../bench/results/WO-M0-03.json) and its header-only [CSV](../../bench/results/WO-M0-03.csv).

## Benchmark gaps

- No owner response or representative real file/task corpus is available; synthetic fixtures cannot fill that gap.
- Inkscape and Donner were unavailable, and Vexel has no compatible SVG render command. Render comparisons are therefore unavailable.
- RSS/resource measurements are missing; M0-03 explicitly lacks peak RSS through its bounded process interface.
- M0-02 generation times do not measure parsing or rendering; its librsvg time measures only oracle re-rendering.
- M0-03 smoke mode produced no trials. There is no M0-03 latency, throughput, output-quality, or resource result.
- These observations are tied to the recorded environment and run artifacts; they do not support broad performance or market conclusions.

## Confidence and limits

Confidence is high for the counts and timings that are directly present in the pinned manifests and result files, and **zero** for user-demand or outcome conclusions because the cited repository contains no owner-provided product evidence. The synthetic corpus provides controlled feature coverage but has unknown representativeness. The 24 goldens are an oracle baseline, not a conformance claim. The M0-02 timings are a small set of generation/oracle observations, not a statistically meaningful product benchmark. M0-03's empty trial set prevents any comparative claim. Machine-readable source hashes below let a reviewer check the exact local evidence bytes.

## Open questions

1. Which real SVG tasks are frequent, costly, or painful enough to justify Vexel?
2. Which five representative files/tasks can the owner provide, and may they be retained for local testing?
3. What success outcome should be measured for each task (fidelity, editability, time, reliability, or another owner-defined result)?
4. Which competitor versions and rendering conditions should define a fair comparison?
5. What memory and latency limits matter on the target Apple Silicon macOS systems?
6. Does the owner want to proceed, pause, pivot, or stop after reviewing the evidence request and any responses?

## Owner evidence request

Please provide **at least five representative real files/tasks** through the owner. For each item, include the source file or a sanitized equivalent, the task you actually need done, the expected successful result, and any sensitivity or retention restriction. Do not send confidential material unless it is cleared for this project.

1. A frequently used, ordinary SVG file and the concrete task performed on it.
2. A complex or large SVG file that currently causes friction, with the failing or slow task.
3. A file with text, fonts, or embedded images and the expected display/editing outcome.
4. A file using gradients, clipping, masks, transforms, or other features important to your workflow, with the expected result.
5. A file or task that represents a distinctive real workflow (for example, import, inspect, edit, or export), including what “done successfully” means.

These are requests, not received evidence. **No responses are available yet.** Once received, record source/permission metadata and task outcomes separately from synthetic fixtures; do not silently replace the current zeros.

## Decision requested

**Owner decision: OPEN.** The owner must choose **proceed, pause, pivot, or stop**; this packet does not make that human decision. The evidence-based recommendation is to **pause any product go/no-go or user-demand claim until the requested real files/tasks and outcomes are reviewed**. Existing already-authorized technical work may continue as engineering work, but it must not be represented as validation of user demand. Record the owner's eventual choice and rationale in a later state/ledger update; this packet does not mark that gate complete.

## Source SHA-256 index

Each local file cited above is recorded here with its full SHA-256. The validator checks existence and exact bytes; roles are in [evidence.json](evidence.json).

| Relative repository path | SHA-256 |
|---|---|
| `bench/corpus/manifest.json` | `a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd` |
| `bench/corpus/manifest.sha256` | `6afa2d4c611c4ce9f5986bdb354ee92163fa8a1808ecbd6dd1408e8d8ae45f33` |
| `tests/golden/manifest.lock` | `a9fb0207a5dbd2ff833b090d3d49fef5c960a56bd1a68809a52430c4382bc1c4` |
| `docs/corpus/PROVENANCE.md` | `f803ceaabd93411d07fad0d6641854f4ebe91f8a76d4fb2e7642efb93f0c97e2` |
| `bench/results/WO-M0-02.json` | `92ec0bb6ca6de2ec071b87f769c8f5095a9db3c55f99dd1669eb3b6bbd3de71d` |
| `artifacts/WO-M0-02/oracle-render.txt` | `403fe194d37ea9cb95ff88604c98593806ea4ba0a7bcc6626851f9277a4705dd` |
| `bench/results/WO-M0-03.json` | `854ff1ed1cacc0483598bd1d4fcb7d1fd118bd47062876759025e4cb929bf1b9` |
| `bench/results/WO-M0-03.csv` | `846f26ace59c2a3d0bc1be61e8a2e24222a8b09ea3575484f7192721c80756d6` |
| `artifacts/WO-M0-03/verify.txt` | `a659a8a8025b74a2d79790328f1de6e2810091cbb52588b4bce3132c27919ee4` |
| `artifacts/WO-M0-01/provenance-check.txt` | `11953a172b5b7ef8fcd54da3016f1e03a5f7079c8285dbd1fce6f7322f396d4f` |
| `artifacts/WO-M0-02/verify.txt` | `5cd9978d2c98cbac1adec83f89064707848b99631863c0c67c0f87409e363167` |
| `artifacts/WO-M0-03/local-checks.txt` | `5c69b7e464f2574cb51382f8fe98837885807c618ddcb3e14bc333a6ec889ada` |
