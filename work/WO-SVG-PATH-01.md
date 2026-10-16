---
id: WO-SVG-PATH-01
revision: 1
status: READY
milestone: SVG-FIDELITY
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 1565ed9e7e999443174d6b73303712ddd544b45811f69edbe5a71ff0103b2b12
parallel_safe: false
network: false
---
# Goal
Accept lexically separable adjacent decimal numbers in SVG path data, including the corpus-observed `-.4.6` form, while preserving source spans, coordinate values, and existing rejection behavior for malformed paths.

# Context
- Follow `OWNER-DIRECTIVES.md`, `docs/CODING_STANDARD.md`, ADR-0018, and the current SVG/Inkscape fidelity goal.
- `artifacts/WO-CORPUS-01/delta.json` and `artifacts/WO-CORPUS-04/render-diagnostics.json` identify large SVG documents refused for path grammar; WO-CORPUS-02 documents `-.4.6` as valid SVG syntax observed in `germany`/`france` inputs.
- `svg_shape_path_operand_boundary` currently permits adjacent path operands beginning with `+` or `-`, but rejects an adjacent `.` even though it can unambiguously begin the next decimal number.
- Preserve exact source bytes; this is a parser acceptance fix, not path normalization or an editor rewrite.

# Files allowed to touch
- `work/WO-SVG-PATH-01.md`
- `src/svg/shape/shape.c`, `src/svg/shape/README.md`
- `tests/svg/shape/path_number_test.c`
- `CMakeLists.txt`, `tests/test_map.json`
- `artifacts/WO-SVG-PATH-01/**`, `STATE.md`, `LEDGER.md`, `ledger.jsonl`

# Protected paths
- Preserve all pre-existing changes. Do not edit corpus files, oracle/golden/reference fixtures, other SVG parser modules, renderer budgets, UI, or MCP code.
- No Linux/Windows code, no GPL editor source, no dependencies, no external corpus bytes copied into the repository.

# Required implementation
1. Update path operand boundary handling to accept an adjacent decimal point only where the preceding number has already terminated and the next token can be parsed as a number. Keep command and arc-flag tokenization correct.
2. Add focused tests proving compact and separated forms decode to identical geometry and numeric values; assert exact source spans for each compact literal and byte-exact source retention.
3. Keep malformed/incomplete command groups rejected with stable path diagnostics. Include at least one negative regression.
4. Document the SVG path-number separator rule and the source-preserving behavior.
5. Map the focused CTest in `tests/test_map.json` if existing prefixes do not already select it.

# Verification
- Build `svgworkbench` and the focused path-number test in Debug, ASan/UBSan, and UBSan presets.
- Run only `svg-path-number-lexing` and `shape-render` CTests in each configuration. Do not run broad UI/E2E, corpus, fuzz, or performance suites in this slice.
- Validate the frozen order and ledger; run `python3 -m json.tool tests/test_map.json`, `git diff --check`, and report changed source sizes.

# Output
Leave changes uncommitted. Record exact commands/results in `artifacts/WO-SVG-PATH-01/verification.txt` and update `STATE.md` and the ledgers. Preserve the larger product goal and identify remaining SVG fidelity work.
