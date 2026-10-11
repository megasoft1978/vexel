# MCP integration continuation (2026-10-11)

Candidate: `integrate/mcp-next`, based on the W16 integration commit. The M0
live host, M1 scene tools, M2 view/animation updates, and M3 agent scenarios
were integrated in this worktree. No frozen work order covers this integration
continuation.

## Integration adjustments

- Successful broker tool responses consistently carry `mode` at the
  structured-content root. The broker adds it once at the response boundary;
  document-specific outputs retain their existing mode value. Semantic broker
  errors also carry the mode. Protocol schema-validation errors occur before
  reaching the broker and remain mode-free.
- The stdio e2e and five agent scenarios assert `mode=headless` for successful
  responses and semantic broker errors.
- Registered the five stdio agent scenarios as `mcp-agent-scenarios` in CTest.
  `VEXEL_MCP_BIN_DIR` points the harness at the active build configuration, so
  Debug, Release, and sanitizer runs exercise their own binaries.
- The five scenarios cover stepwise drawing, SVG animation, saved source bytes,
  and exact 64x64 PNG comparisons against checked-in PPM references.

## Verification

- `./dev test-fast`: 303 passed, 0 failed, 0 errors, 0 skipped; ledger validates
  with 412 entries. The suite was rerun after the scenario process cleanup.
- `./dev t -k mcp`: build passed, 7/7 MCP CTests passed, and 2/2 Python MCP
  tests passed.
- Debug CTest excluding only `native-shell-goldens`: 81/81 passed.
- Release CTest excluding only `native-shell-goldens`: 81/81 passed.
- ASan+UBSan MCP selection (scene, protocol, parser fuzz, stdio e2e, agent
  scenarios, features, live viewer host): 7/7 passed.
- UBSan MCP selection: 7/7 passed.
- Full Debug and Release CTest reruns after the response-mode fix: 81/81 each,
  excluding only `native-shell-goldens`.
- Final `./dev test-fast`: 303 passed, 0 failed, 0 errors, 0 skipped; ledger
  validates with 412 entries.
- `./dev mcp-fuzz`: parser campaign 200,000 cases (50,013 reached argument
  validation, seed 12648430) and viewer-host campaign 50,000 cases (seed
  20261011); both passed.
- `git diff --check`, staged diff check, and `git diff --cached --check`: pass.
  Source audit checked 301 C/H/Objective-C files; all are below 800 lines.

The only excluded CTest is `native-shell-goldens`. The active BetterDisplay
configuration presents the offscreen AppKit bitmap at 2x (1200x1300 rather
than the 600x650 fixture). Downsampling still differs from the fixture, and
the current output is byte-identical to a clean W16 baseline rendered under
the same display configuration. The fixture was not regenerated. All other
Debug and Release CTests passed, including native viewer smoke and index replay.

A test-only 1x-window experiment produced 600x650 output but still differed
from the frozen image in a transient inline-button hover highlight. Dark-mode
images also use a different `windowBackgroundColor` value on the current macOS
SDK. The experiment was reverted; no native UI source or golden changed.
