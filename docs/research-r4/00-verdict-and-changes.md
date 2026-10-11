# Round 4 verdict and changes

**Decision: conditional narrow GO.** Keep C17, the custom-drawn UI, minimal dependencies, Mac-first delivery, basic editor scope, and the wedge: *a fast, faithful large-SVG workbench*. Animation remains after the static editor. An AI-only team lowers the cost of implementation and test breadth; it does not make SVG fidelity, platform behavior, product judgment, or evidence cheap. The wedge stays narrow because verification and human product decisions are now the critical path.

The new model does **not** justify a broader first release. It makes prototypes and test harnesses cheaper, but expands the amount of output that must be independently checked. Donner remains the closest competitor; its status does not change, so the same black-box, same-corpus comparison remains a gate.

## What changed from Round 3

- Replace “one senior engineer + LLM help” with a planner/executor system: Claude Sonnet writes/approves bounded work orders and reviews compact evidence; Codex `codex exec` with GPT-6 Luna implements, tests, benchmarks, and reports. More than 90% of model tokens should be Luna. Fresh session per work order is default; resume only an interrupted WO with unchanged spec and worktree.
- Add an independent Luna verifier on a separate fresh session and read-only worktree. Machine checks are authoritative for objective criteria; the verifier is adversarial and cannot edit. Sonnet reviews only the schema-validated verifier verdict, delta summary, and `git diff --stat` unless escalated.
- Treat every implementation claim as a testable artifact. Frozen acceptance tests, generated counterexamples, reference renderers, screenshots, benchmark raw data, and source hashes make “done” observable. Human gates remain for product value, visual quality, accessibility with real assistive tech, IME, and release.
- Revise cost model: Luna text generation is inexpensive at published API rates, but `codex exec` on a ChatGPT account uses plan-specific shared usage/credit limits rather than a public unlimited token pool. Budget is therefore run-count and allowance based. Purchase/upgrade decisions are owner-controlled.

## Verdict and gates

Proceed through M0–M3 as a **6–8 week evidence proof**, with a hard stop before broad feature work if the benchmarks, source-preservation proof, or user signal fail. Donner remains the nearest direct competitor; AI execution does not remove the requirement to benchmark it on the same pinned corpus and tasks. Do not claim a speed edge until reproducible head-to-head results show one.

Proposed M3 pass conditions (estimates, not published facts): first visible pixel under 2 s for a 100 MB map-like SVG on named Apple Silicon; no main-thread pause over 100 ms during indexing; visible pan/zoom p95 under 33 ms for the agreed corpus; no-op save preserves source bytes; at least five target users provide representative files/tasks and agree to pilot; no known critical data-loss or memory-safety issue. Kill or narrow to a read-only viewer if two measured architecture iterations miss the performance gate, preservation requires eager normalization, Donner matches the primary tasks within 20%, or interviews fail to confirm top-three pain.

## Revised schedule and cost (planning estimates)

| Scope | Optimistic | Expected | Pessimistic | Main uncertainty |
|---|---:|---:|---:|---|
| M0–M3 proof | 4–5 weeks | 6–8 weeks | 10–14 weeks | Mac UI automation and real corpus/legal access |
| Mac basic editor after proof | 5–8 months | 9–15 months | 18–28 months | SVG style/text/stroke fidelity and round-trip model |
| Linux + Windows ports after Mac beta | 3–5 months | 6–10 months | 12–18 months | IME, accessibility, compositor and driver variance |
| Animation after static-editor gate | 4–7 months | 8–14 months | 18–30 months | timeline semantics, SMIL/CSS scope, interaction polish |

Expected schedule assumes 1–2 Luna work orders/day on average after bootstrap, strict serial integration, occasional allowance pauses, and owner review latency; agents can run continuously but the project is not parallelizable without limit. A work order is typically 1–4 Luna runs including verifier; difficult graphics work can take 5–10. Sonnet planning/review target: 1–3k tokens per ordinary WO, 4–8k for milestone/architecture changes. These are estimates. The proof maps to roughly 75–105 Luna sessions and 45k–90k Sonnet tokens; API-list-price equivalent is only about $4–14 for the estimated Luna text tokens, excluding tools and not representative of Codex-plan allowance consumption. GitHub Actions rates at research time are $0.006/min Linux x64, $0.010/min Windows x64, and $0.062/min macOS standard ([official rates](https://docs.github.com/en/billing/reference/actions-runner-pricing)); availability, included minutes, and rates can change. Codex plan allowances vary and should be checked in the [usage dashboard](https://help.openai.com/en/articles/11369540-using-codex-with-your-chatgpt-plan).

## Human gates (few, explicit)

1. M0: confirm target user/corpus and whether the evidence warrants continuing.
2. M3: review 5–8 representative screenshots plus a live Mac build; decide whether basic editor scope remains justified.
3. Before beta: hands-on workflow/feel review; VoiceOver on real macOS; real CJK IME composition; sign/notarize and release decision.
4. Before Linux/Windows support claims: owner-selected real hardware/compositor/IME/accessibility spot check; CI alone does not establish support.
5. Before animation: separate product-demand and scope gate.

## Top risks

Reward hacking against visible tests; subtle C memory errors; wrong visual oracle; API hallucination; benchmark noise/thermal throttling; shared Codex limits; zero-human loss of codebase comprehension; malware/license contamination from autonomous dependency/source browsing. Controls and kill rules are in `06-risks-and-safety.md`.
