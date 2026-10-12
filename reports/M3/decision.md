# M3 viewer proof and owner decision

> Review material is the recorded screenshots, semantic snapshots, and scripted checks linked below. This packet does not claim a live customer pilot.

## Viewer review

- [Navigation frame 1](../../artifacts/WO-M3-04/navigation-frame-0000.png)
- [Navigation frame 2](../../artifacts/WO-M3-04/navigation-frame-0001.png)
- [Semantic snapshot: active progress](../../artifacts/WO-M3-04/semantics-0000.json)
- [Semantic snapshot: cancel request](../../artifacts/WO-M3-04/semantics-0001.json)
- [Semantic snapshot: terminal state](../../artifacts/WO-M3-04/semantics-0002.json)
- [Navigation checks and projection evidence](../../artifacts/WO-M3-04/navigation-tests.json)

## Recorded evidence

- Hardware/compiler: Mac Studio (Mac17,14), Apple M5 Max, macOS 27.0, Apple clang version 21.0.0 (clang-2100.3.34.2).
- BOOT-08 protocol: 5 cold-order + 10 warm-order runs per size. Cold/warm labels are run order; filesystem cache was not flushed.
- Final BOOT-08 values (p95 first preview; median full index; maximum step; peak RSS):

| Case | First preview p95 | Full index median | Max step | Peak RSS |
|---|---:|---:|---:|---:|
| 100MiB | 274657958 ns | 343656583 ns | 218834 ns | 139526144 bytes |
| 10MiB | 28098625 ns | 44385250 ns | 273208 ns | 37912576 bytes |

- Overall final gate: **fail**. Final 100 MiB RSS is 139526144 bytes against an absolute limit of 524288000 bytes: the absolute limit passes, while the relative 5% baseline gate fails after two consecutive violations.
- Pan timing: unavailable — navigation test validates event behavior but does not time presentation
- App bundle: unavailable — no macOS app-bundle packaging target exists in this work order
- M2-04: 3 byte-equal round trips and 4 failure-atomicity cases passed.
- Fuzz: M2-03 campaigns completed 2158912 and 42108425 cases; M2-05 completed 1000000 cases with 0 crashes and 0 sanitizer reports.
- Navigation: 11 recorded checks passed; projection workload 1000000 items, checksum `804ede4d47af0f15`. No pan-to-present timing is claimed.
- Competitors: Inkscape and Donner unavailable; 0 render trials; Vexel CLI has no compatible SVG render command.
- Synthetic corpus: 24 fixtures and 24 goldens. These are engineering coverage, not customer evidence or complete SVG compatibility.
- Owner files/tasks: 0; interviews: 0; outcomes: 0; pilots: 0.

## Owner decision

**Decision: OPEN.** No choice is recorded by this packet.

Please provide five representative real files/tasks, each with the task, expected successful result, and any sensitivity or retention limits.

- **proceed** — Authorize M4/M5 planning after reviewing real representative tasks; technical progress alone does not validate demand.
- **pause** — Hold M4/M5 until five representative files/tasks and outcomes are reviewed; preserves the current evidence boundary.
- **pivot** — Redirect scope based on owner-provided tasks; requires evidence identifying a better target workflow.
- **stop** — End product work if the evidence does not justify further investment; retain the packet for the decision record.

## Confidence and limitations

High confidence for counts and measurements that are directly recorded in hash-pinned source artifacts; all source bytes and derived values are checked locally.

No evidence; zero owner/user/pilot records.

Measurements describe the named Apple M5 Max / macOS 27.0 / Apple Clang 21.0.0 environment and synthetic workloads only.

## Checked source SHA-256

Every packet source below is repository-relative and verified by the offline validator.

| Repository path | SHA-256 |
|---|---|
| `artifacts/WO-BOOT-08/gate-contract.json` | `bc797fad33ede0c898d040deb06fdda0edd2ba19b48b3ad41321d7f055c96f2c` |
| `artifacts/WO-BOOT-08/regression-run-01.json` | `65a18aacf615ae026940afda93a8c7762935432b2f381af80721359da24afe05` |
| `artifacts/WO-BOOT-08/regression-run-02.json` | `a9e63e81454ad47c9d223cc72169925de722f8917ea6c4fdd6625a20288a1506` |
| `artifacts/WO-M0-01/provenance-check.txt` | `11953a172b5b7ef8fcd54da3016f1e03a5f7079c8285dbd1fce6f7322f396d4f` |
| `artifacts/WO-M0-02/oracle-render.txt` | `403fe194d37ea9cb95ff88604c98593806ea4ba0a7bcc6626851f9277a4705dd` |
| `artifacts/WO-M0-02/verify.txt` | `5cd9978d2c98cbac1adec83f89064707848b99631863c0c67c0f87409e363167` |
| `artifacts/WO-M0-03/local-checks.txt` | `5c69b7e464f2574cb51382f8fe98837885807c618ddcb3e14bc333a6ec889ada` |
| `artifacts/WO-M0-03/verify.txt` | `a659a8a8025b74a2d79790328f1de6e2810091cbb52588b4bce3132c27919ee4` |
| `artifacts/WO-M2-03/pr-smoke.json` | `551fecd5b139ab58c6a035ba933d198dcc88fc33330a9fe6dbfd45a2017931dd` |
| `artifacts/WO-M2-03/scheduled-campaign.json` | `746031ac58e248c2a2d4590ae11e839f86eaa9dfb7948b8f37e0e54afda0f2c2` |
| `artifacts/WO-M2-04/failure-atomicity.json` | `44f644bcc4a279be597243ab843eafbfa104cafc9b4fcf7a5abd8fa974cf5ca0` |
| `artifacts/WO-M2-04/roundtrip.json` | `83ba83a9236eb3ae05d2f7c791c8b0f6f4b69d4a5bb8d423c3116d9765c53ecc` |
| `artifacts/WO-M2-05/fuzz-report.json` | `c544147e95da12bb267e68af98f381ebe6b90a69a0773ab73ff00fd5d110e37c` |
| `artifacts/WO-M3-01/progressive-replay.json` | `e589f05056582ab98d1de9f1cdff5be1756969f1293e66337e1414e12f1af789` |
| `artifacts/WO-M3-04/navigation-frame-0000.png` | `f6221b0a17dc1ad5cf6675f8ea1f455c99d13226021f94461af1b818a33348a9` |
| `artifacts/WO-M3-04/navigation-frame-0001.png` | `642b9b9640e6504ec05ce2642a01b9bf85ff0e891afade7361c00135c977f0fc` |
| `artifacts/WO-M3-04/navigation-golden-manifest.json` | `311a9c14328193a87435c7a0b881defd9fce23ba3d7a7a294185ed87d9b3b887` |
| `artifacts/WO-M3-04/navigation-tests.json` | `0dfe40c8f02a4243a3b8d7ba87a514082efd50a3e691bb1df6d61d44f0ec34ee` |
| `artifacts/WO-M3-04/semantics-0000.json` | `b1fe92734296ac8dd2463df365dea1b5a19b195e9e8eb2d5413c293348ad6506` |
| `artifacts/WO-M3-04/semantics-0001.json` | `556b4301f050e285ea443c2a1adb5de7ac059e2a9cd8e7d068929e0f8897fe9c` |
| `artifacts/WO-M3-04/semantics-0002.json` | `8f09bbde0cbb58a11696b05b08a3046bbc9e5e071220debb1edb88abdd521c42` |
| `artifacts/WO-M3-04/semantics.json` | `223b9456dd000f127e4b2a077c92fe4bd6796fbbb7b4b091a5d2191574d51648` |
| `bench/corpus/manifest.json` | `a05dcf0da95bd7c53ce6faec6704f7b739d6cbd9cf829dfff74b50dfb8359bbd` |
| `bench/corpus/manifest.sha256` | `6afa2d4c611c4ce9f5986bdb354ee92163fa8a1808ecbd6dd1408e8d8ae45f33` |
| `bench/results/WO-BOOT-08.json` | `a9e63e81454ad47c9d223cc72169925de722f8917ea6c4fdd6625a20288a1506` |
| `bench/results/WO-M0-02.json` | `92ec0bb6ca6de2ec071b87f769c8f5095a9db3c55f99dd1669eb3b6bbd3de71d` |
| `bench/results/WO-M0-03.csv` | `846f26ace59c2a3d0bc1be61e8a2e24222a8b09ea3575484f7192721c80756d6` |
| `bench/results/WO-M0-03.json` | `854ff1ed1cacc0483598bd1d4fcb7d1fd118bd47062876759025e4cb929bf1b9` |
| `bench/results/WO-M3-01.json` | `f2d7d9cd7c95bd43d185ff0a9d994d21d1c5b109ece06d0eb41d5d58cd73569a` |
| `docs/PERF.md` | living input, not byte-frozen |
| `docs/adr/0001-product-and-platform-locks.md` | `143ecfd9821b69b23d732eb774b576192c82eec10562a8942e1a454158429ed0` |
| `docs/adr/0006-m1-04-owner-quiescence.md` | `45f9e99b64d3184fc7fce5e9a6823b0fd758fd779b44ae6d1a9b120c7a9f31fa` |
| `docs/corpus/PROVENANCE.md` | `f803ceaabd93411d07fad0d6641854f4ebe91f8a76d4fb2e7642efb93f0c97e2` |
| `reports/M0/evidence.json` | `d4eeadf18d6da4001e0b6f2d531aec419669edc8c028d92ae481dcd500db082b` |
| `reports/M0/product-evidence.md` | `2336211d8e9fdef2cf5731b5cc31e15e6799ba619f75264b6379bf41b28a188f` |
| `tests/golden/manifest.lock` | `a9fb0207a5dbd2ff833b090d3d49fef5c960a56bd1a68809a52430c4382bc1c4` |
| `work/WO-BOOT-08.md` | `f7fbc767f475c0a4b264987bdd54719e8e87e4910fee4e04bffef12f5790c98f` |
| `work/WO-M3-01.md` | `54bd664e5061aede7a133b07869e96f9401027cc7f404b86a11ff24b8230e748` |
| `work/WO-M3-02.md` | `d2547fe2a339e20a7e1c01c7468010452750ee73166c494699a0a130af03bbdf` |
| `work/WO-M3-03.md` | `4f191c263623fe02d8ef51c7ac2a71ed80ee6fef94b00a0a7055510b21de88fe` |
| `work/WO-M3-04.md` | `9fd77fed0650fb718b209626b43641b99fbadf9654fbe6488555f0206289a0db` |
