# AI execution risks, security, and Sonnet token budget

## Risk register

| Risk | Likelihood / impact | Mitigation | Stop trigger |
|---|---|---|---|
| Reward hacking (tests weakened, fixtures special-cased) | High / critical | Frozen oracle/corpus hashes; feature WOs cannot edit tests; separate baseline WOs; mutation audit; hidden owner-held fixtures; differential references; verifier checks assertions and hardcoded IDs | Any weakened assertion, fixture special-case or claim/artifact mismatch blocks merge |
| Silent visual/source regression | Medium / critical | PNG goldens/heatmaps, unknown metadata byte tests, no-op hashes, differential render, source-delta allowlist, fault-injected save | Silent data drop or unreviewed baseline change |
| Context/spec drift | High / high | Immutable WO revision/hash, STATE + ADRs, one WO/session, fresh verifier, module README/contracts | Repeated misunderstandings or unexplained boundary changes |
| Hallucinated APIs | Medium / high | Official versioned docs, installed SDK headers, compile probe, verifier checks symbols/build | API cannot compile against pinned SDK |
| C memory safety | Medium-high / critical | Bounded slices, checked arithmetic, arenas, ASan/UBSan in dev, fuzzing, analyzer, TSan, minimized regressions; evaluate -fbounds-safety experimentally | Any sanitizer issue blocks merge |
| Perf benchmark noise | High / high | Fixed corpus/hardware, repeated trials, thermal metadata, median/interval, two-run comparison | >10% regression beyond noise on two runs |
| Rate limits/model drift | Medium / high | Pin CLI/model where possible; check /status + dashboard; run cap; log model; pause instead of silent fallback | Pause >5 business days or <1 WO/day for 2 weeks |
| Runaway cost | Medium / high | Hard timeout, two retries, weekly run/time ceiling, metering if exposed, no API billing fallback | Unknown metering plus repeated long jobs |
| Unsafe shell/network | Medium / critical | Dedicated unprivileged account, isolated checkout, no credentials, network denied by VM/firewall, workspace-write, verifier read-only, ephemeral VM for unknown code | Unexpected access/exfiltration signal |
| GPL contamination | Low-medium / high | No Inkscape source in context; binary-only benchmarking; provenance/SBOM/license and similarity scans; counsel before distribution | Unexplained similarity or missing provenance |
| Dependency drift | Medium / high | Exact commit/hash, SBOM, no floating tags, CVE checks, reviewed fetch, adapter boundary | Hash mismatch or unpatched critical CVE |
| Nondeterministic build | Medium / medium | Pinned compiler/SDK, lockfiles, no configure downloads, repeat clean build | Unexplained output mismatch |
| Zero-human bus factor | High / high | Architecture map, READMEs/contracts, ADRs, release runbook, fresh-agent architecture exam | Two fresh runs cannot explain/repair critical module |
| UX/accessibility judgment | High / high | UI driver, screenshot goldens, semantic tree, Luna image review, owner gates | Owner cannot complete workflow or core controls inaccessible |
| Local Mac bottleneck | High / medium | Serial integration, hosted CI, one platform workload, benchmark window | Hardware pause >1 week |
| Portability false confidence | Medium / high | Native Linux/Windows CI; Weston is protocol smoke only; Wine is not support evidence | Claim based only on emulation |

## Sandbox blast radius

CLI sandboxing is defense in depth, not isolation from hostile build scripts. Routine executor uses a dedicated macOS account with repository and toolchain only: no Documents, SSH/cloud tokens, browser profile, production data, signing identity. Use per-WO worktrees and workspace-write. Deny network at VM/firewall level; model-service egress may use a controlled proxy, while builds/tests run offline. No broad full-disk access. Verifier gets artifact copies and read-only sandbox. Fuzzing third-party parsers and unknown dependency builds run in disposable VM snapshots. Keep credentials in host broker, never environment/worktree. Mac isolation is limited; Linux VM/UTM helps with hostile builds but cannot validate Cocoa/Metal. Never use sandbox-bypass flags routinely.

## License and provenance controls

- Never provide Inkscape source, copied snippets or source search results to the executor. Benchmark released binaries only.
- Each vendored component records URL, exact version/hash, SPDX/license text, transitive dependencies, local modifications, build flags and maintenance owner.
- Adapted algorithms record permissive source/paper/version. Similarity scanning is an alert, not a legal conclusion; ambiguous cases go to counsel.
- License scan and SBOM run before distribution. Generated code is not automatically license-free.
- Do not copy code without license review.

## Sonnet token-saving rules

Sonnet writes a 1–2 page WO, architecture changes/ADRs, exceptional diagnoses, milestone review and product gate memo. It should not read whole repos, raw logs, transcripts, unfiltered benchmark CSVs or routine full diffs. Begin with structured digest; request targeted excerpts only when needed.

Luna drafts WOs from roadmap, expands tests, implements, triages logs, summarizes artifacts, drafts module READMEs and finds potential review issues. Sonnet checks Luna's draft against locked decisions and AC quality. Luna cannot approve its own spec or verifier result.

Summarizer emits base/head SHA, paths/diff stat, pass/fail IDs, failed command with <=20 lines, benchmark medians, image paths/hashes, risk flags and contract changes. Cache stable context as pointers and include relevant module docs only. Use official-doc URLs/version pins, not long excerpts. Sonnet review cap: 2k tokens/WO and 8k/milestone.

### Budget model (estimate)

20 WOs/month: 20 × 1.5k planning/approval + 20 × 1k review + 4 × 5k milestone/ADR ≈ 60k Sonnet tokens/month; 100k hard cap. Luna: 20 × (1.5 executor attempts + 1 verifier) ≈ 50 runs/month; 20k–60k output tokens/run is rough and tool transcripts can dominate. Proof estimate: 45k–90k Sonnet tokens and 75–105 Luna sessions. Track usage weekly. Do not multiply API list prices by ChatGPT-plan Codex use and call that the invoice.

## State recovery

Every run stores event metadata and artifact hashes. After failure inspect ledger, main SHA, worktree, WO revision, run ID and session ID. Resume only the same session after checking diff; otherwise start fresh with short summary and same spec hash. Git and state are durable truth, not transcripts. Daily report: open WOs, orphan worktrees, failed gates, stale STATE, quota status. New agents must navigate STATE → WO → module README → tests without old transcripts.
