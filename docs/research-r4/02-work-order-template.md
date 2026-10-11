# Work order format and schemas

A WO is the only authorization for code changes. It is written by Sonnet, immutable once marked `READY`, and specific enough that Luna can act without product interpretation.

## Exact template

```markdown
---
id: WO-XXX-YY
revision: 1
status: READY
milestone: Mx
owner: sonnet-5.5
executor: codex-gpt-6-luna
base_sha: <40-hex>
spec_sha256: <wrapper fills>
parallel_safe: false
---
# Goal
One observable result; one sentence.

# Context pointers
- Read: `AGENTS.md`, `STATE.md`, `path/header.h`, `path/README.md`
- Relevant ADRs: `docs/adr/NNNN.md`
- Frozen tests/oracles: IDs and paths
- External docs: official URL + version/date; never infer API from memory

# Existing contracts and invariants
Ownership/lifetime, threading, limits, stable IDs, serialization, errors.

# Files allowed to touch
- `path/glob` (create/update)

# Protected paths
- `tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`
- Other excluded paths:

# Required implementation
Numbered bounded behavior. State failure cases and resource limits.

# Acceptance criteria (machine-checkable)
Each AC has ID, exact command, expected exit/status/threshold, evidence file.
- AC-1: `...`

# Verification sequence
1. build command
2. focused tests
3. full required tests
4. sanitizer/fuzzer/benchmark command
5. report commands and artifact paths

# Non-goals
Explicit exclusions.

# Budget
Executor max runs: 2; verifier: 1; elapsed hard limit: 120 min; output-token soft cap: 15000; dependency/network: none.

# Failure and escalation
Retry conditions; stop triggers; what question/evidence to return.

# Output
Commit convention; expected changed files; report schema path; JSON output only at final response.
```

## Executor final report JSON Schema

```json
{
  "$schema":"https://json-schema.org/draft/2020-12/schema",
  "type":"object","additionalProperties":false,
  "required":["wo_id","status","summary","base_sha","head_sha","changed_paths","commands","tests","commit","blockers"],
  "properties":{
    "wo_id":{"type":"string"},
    "status":{"enum":["complete","partial","blocked","failed"]},
    "summary":{"type":"string","maxLength":1200},
    "base_sha":{"type":"string"},"head_sha":{"type":"string"},
    "changed_paths":{"type":"array","items":{"type":"string"}},
    "commands":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["cmd","exit_code","duration_s","evidence"],"properties":{"cmd":{"type":"string"},"exit_code":{"type":["integer","null"]},"duration_s":{"type":"number","minimum":0},"evidence":{"type":"string"}}}},
    "tests":{"type":"array","items":{"type":"object","additionalProperties":false,"required":["id","status","evidence"],"properties":{"id":{"type":"string"},"status":{"enum":["pass","fail","skip","not_run"]},"evidence":{"type":"string"}}}},
    "commit":{"type":["string","null"]},
    "usage":{"type":"object","properties":{"input_tokens":{"type":["integer","null"]},"output_tokens":{"type":["integer","null"]},"cli_version":{"type":"string"}},"additionalProperties":false},
    "blockers":{"type":"array","items":{"type":"object","required":["category","detail","evidence","needed"],"properties":{"category":{"type":"string"},"detail":{"type":"string"},"evidence":{"type":"string"},"needed":{"type":"string"}},"additionalProperties":false}}
  }
}
```

## Verifier verdict JSON Schema

```json
{
  "$schema":"https://json-schema.org/draft/2020-12/schema",
  "type":"object","additionalProperties":false,
  "required":["wo_id","verdict","base_sha","head_sha","acceptance","scope_findings","risk_flags","summary"],
  "properties":{
    "wo_id":{"type":"string"},"verdict":{"enum":["pass","fail","uncertain"]},
    "base_sha":{"type":"string"},"head_sha":{"type":"string"},
    "acceptance":{"type":"array","items":{"type":"object","required":["id","result","evidence"],"properties":{"id":{"type":"string"},"result":{"enum":["pass","fail","not_testable"]},"evidence":{"type":"string"},"detail":{"type":"string"}},"additionalProperties":false}},
    "scope_findings":{"type":"array","items":{"type":"object","required":["path","finding","severity"],"properties":{"path":{"type":"string"},"finding":{"type":"string"},"severity":{"enum":["info","low","high","critical"]}},"additionalProperties":false}},
    "risk_flags":{"type":"array","items":{"type":"string"}},"summary":{"type":"string","maxLength":1600}
  }
}
```

## Ledger JSONL entry schema

```json
{"ts":"RFC3339 UTC","wo_id":"WO-M0-01","event":"ready|run_started|run_finished|verified|accepted|rejected|merged|blocked|superseded","revision":1,"base_sha":"...","head_sha":"...","run_id":"...","session_id":"...","model":"gpt-6-luna","cli_version":"...","duration_s":0,"usage":{"input_tokens":null,"output_tokens":null},"status":"...","artifacts":["relative/path"],"note":"<=500 chars"}
```

`ledger` rejects unknown fields, invalid event transitions, duplicate run IDs, non-UTC timestamps, and WOs without a frozen spec hash. The append is atomic under a lock; the verifier cannot mutate ledger state.

## Filled example: WO-M0-01

```markdown
---
id: WO-M0-01
revision: 1
status: READY
milestone: M0
owner: sonnet-5.5
executor: codex-gpt-6-luna
base_sha: <main SHA>
parallel_safe: false
---
# Goal
Create a reproducible C17 project skeleton that configures and builds a no-op CLI on the current Mac with strict warnings.

# Context pointers
Read prior decision `out3/plan/08-decisions-adr.md` (locked C17/custom UI/minimal deps/Mac-first); this packet `04-llm-aware-design-changes.md`; no external source code. macOS SDK and CMake docs only if an API is needed.

# Existing contracts and invariants
C17; CMake; no third-party runtime dependency; deterministic configure/build; warnings as errors; generated files under `build/` only.

# Files allowed to touch
`CMakeLists.txt`, `cmake/**`, `src/base/**`, `app/main.c`, `AGENTS.md`, `README.md`, `tests/**` (not `tests/oracle/**`), `.gitignore`.

# Protected paths
`tests/oracle/**`, `tests/golden/manifest.lock`, `bench/corpus/manifest.lock`, `out3/**`.

# Required implementation
Add top-level CMake project with explicit C17, a minimal executable printing version and exiting, warning profile for Clang, and documented configure/build commands. Do not add UI frameworks or libraries.

# Acceptance criteria
- AC-1 `cmake -S . -B build -DCMAKE_BUILD_TYPE=Debug` exits 0.
- AC-2 `cmake --build build --parallel 2` exits 0 with all project warnings treated as errors.
- AC-3 `./build/bin/svgworkbench --version` prints stable `svgworkbench 0.0.1` and exits 0.
- AC-4 configure/build twice from clean build dirs has no generated source changes; `git status --porcelain` is empty before commit.
- AC-5 `cmake --build build --target help` has no accidental dependency download targets.

# Verification sequence
Run all AC commands and save `artifacts/WO-M0-01/build.txt`; report compiler/CMake versions. Inspect `git diff --check` and changed-path allowlist.

# Non-goals
No Cocoa window, renderer, XML parser, CI provider, vendored library, benchmark, network fetch.

# Budget
2 executor runs, 1 verifier, 90 min hard limit, 10000 output tokens, no network.

# Failure and escalation
Stop if current toolchain cannot express a required warning cleanly; report exact compiler diagnostic and proposed fallback. No weakening warnings without Sonnet approval.

# Output
One commit `build: scaffold C17 project`; executor JSON per schema; verifier JSON per schema.
```
