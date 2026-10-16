---
id: WO-RUST-APPLE-PREVIEW-PRESENTATION-01
revision: 1
status: READY
milestone: MCP-LIVE-PREVIEW
owner: project-owner
executor: gpt-6-luna
base_sha: 19234a036ed6a8d40c2ccdaec9b8186806d70eca
spec_sha256: 33835a3d73e4371045f27cd1c9f5390011b6c1dd55277b46ca00612a74c799f9
parallel_safe: false
network: false
---
# First native preview frame presentation repair

The first MCP-driven preview run initializes successfully, but its first
`document_list` request closes the server because the app cannot present its
initial native RGBA frame (`PresentationUnavailable`). Diagnose and repair the
native view/presentation setup so the preview can keep one window open, service
MCP calls, and visibly publish each accepted shape before returning success.

## Scope
- Preserve the public `Window::present_rgba` API and current frame ownership.
- Correct the native view/layer setup for the window's custom drawing view.
- Keep the normal app behavior and existing presentation semantics intact.
- Keep diagnostics on stderr and useful; never write logs to MCP stdout.
- Validate with focused package checks/tests and one requested live MCP preview
  run. Do not run the broad UI/E2E suite.

## Allowed paths
- `rust/crates/vexel-platform-apple/src/native_window/app.rs`
- `rust/crates/vexel-app/src/preview_app.rs` (diagnostic detail only)
- `artifacts/WO-RUST-APPLE-PREVIEW-PRESENTATION-01/**`
- `.cache/**` for candidate/build evidence only

## Protected paths
All other source, manifests, lockfiles, tests/oracles, work orders, `STATE.md`,
ledgers, HEAD/index, legacy C/Objective-C. No dependencies, new platform support,
GUI automation, commit/staging/reset/deletion, or broad UI test suite.

## Acceptance
- A live preview launch no longer exits on its first blank frame.
- MCP initialize, document listing, info, one rectangle/ellipse add and its
  response ordering work against the same window.
- Focused `cargo +stable check/test -p vexel-platform-apple -p vexel-app
  --locked --offline`, scoped formatting, and `git diff --check` pass.
- Record the exact native runtime evidence and remaining GUI/visual limitations.
- Independent review before accepting the platform change.

This is a runtime repair for the interim preview only. The full Vexel goal and
final broad UI/E2E gate remain active.
