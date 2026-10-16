---
id: WO-RUST-NATIVE-BINDINGS-AUDIT-01
revision: 1
status: READY
milestone: RUST-MIGRATION
owner: project-owner
executor: codex-gpt-6.1-sol
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: c6c015d1b3ab478729475fb46d468d97c2cbebe6a0e9ab5e7880589de4103da6
parallel_safe: true
network: true
---
# Goal
Resolve the accepted native desktop proposal's dependency prerequisite using
primary-source evidence. Produce an adoption-ready pinned minimal-feature plan
for objc2, objc2-encode, Foundation/AppKit/CoreFoundation/CoreGraphics/CoreText/
Metal/QuartzCore and block2 only if justified, including actual transitive
closure, licenses and macOS13 SDK availability. Audit first; do not adopt any
dependency, fetch via Cargo, install, change source/manifests or claim GUI success.

# Files allowed to touch
- `work/WO-RUST-NATIVE-BINDINGS-AUDIT-01.md` (coordinator freeze only)
- `artifacts/WO-RUST-NATIVE-BINDINGS-AUDIT-01/**`
- `docs/product/rust-native-bindings-audit.md`
- `STATE.md`, `LEDGER.md`, `ledger.jsonl`, `docs/product/rust-port-goal-and-plan.md` (coordinator only)

# Required evidence
Read OWNER-DIRECTIVES/AGENTS, ADR0022/0023/0024/0025, accepted native proposal,
original platform window/shell/input/menus/accessibility/text/Metal headers and
relevant excerpts. Audit all required native capabilities, not only opening a
blank window. Native pointer/message boundary stays platform-owned. Recommend
exact registry version pins and default-features=false feature lists by shell,
text/presenter, menus/AX/dialog/clipboard/preferences scopes. Verify current
primary registry metadata, published Cargo manifests/licenses/SDK provenance,
source retention/callback/MainThreadMarker/block/panic/exception APIs and
Rust1.99 compatibility. Record full resolution rules and minimal transitive
closure; if impossible without future compilation say exactly what remains.
Public metadata/manifests/docs/licensed binding source archives may be fetched
with stdlib urllib into own artifacts only (bounded HTTPS, no secret/private
queries, honor errors, no retries). Do not fetch/read/copy GPL SVG editor source.
No account/config/network external SVG resources. No Cargo downloads or package
installation; downloaded research archives are not adopted dependencies.
Use installed macOS SDK headers to verify exact API availability, record SDK
path/version and post13 symbols requiring guards/alternatives. Do not claim
macOS13 runtime compatibility from current-host compilation or current SDK only.
Record source URLs/date/hash/version/license texts and a symbol-feature matrix.
No speculative verified-license/MSRV/availability claims without exact evidence.

# Protected paths and verification
All product code, other docs/artifacts/orders, Cargo manifests/locks/toolchain,
index/HEAD and603legacy files. No recursive agents/stage/commit/reset/delete,
unsupported OS, C shim, installs, paid API/credits or broad tests. One actual
Standard subscription Sol6.1Medium CLI process; scope only own report/evidence.
python3 scripts/wo_validate.py WO-RUST-NATIVE-BINDINGS-AUDIT-01 --ready
python3 scripts/ledger.py validate ledger.jsonl
git diff --check
Root checks exact preservation and primary evidence hashes, then independent
focused review. This report informs a separate dependency ADR/adoption order.

# Output
Report<=1800words plus machine-readable pins/features/symbols/licenses/source
provenance (include unresolved items), raw bounded primary evidence, exact
verification and result<=350words. No dependency or native implementation
acceptance. Stop/checkpoint allowance/network errors without retry loops.
