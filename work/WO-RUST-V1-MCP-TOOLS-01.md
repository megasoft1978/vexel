---
id: WO-RUST-V1-MCP-TOOLS-01
revision: 1
status: READY
milestone: V1-SVG-COMPOSITION
owner: project-owner
executor: gpt-6-luna
base_sha: b10a78b3231c70901e55edafd8057064a4219716
spec_sha256: 23a7df43539fd8dec20e7f4100102fa14ce5b7a6f4cb39bc15fc7f9e4d109808
parallel_safe: false
network: false
---
# Route the walkthrough MCP tools into the live app session

## Goal
Today `LiveBackend` (`vexel-app/src/preview.rs`) and `McpHost`
(`session/mcp.rs`) route only 9 of the 43 declared `vexel-protocol` tools
(`document_list`, `document_info`, `element_add` rect/ellipse, `undo`, `redo`,
`animation_list`, `animation_add`, `animation_control`, `export_animation`).
Route the tools the V1 walkthrough needs so a scripted client can run the
ADR-0029 flow over MCP against the one shared `AppDocument`, and give
`vexel-app` a library target so integration tests can link. No new product
features, no new tool names.

## Dependencies
INTEGRATE-01, SAVE-01 and PLAYBACK-01 integrated (HEAD b10a78b3). Read first:
`preview.rs` (`parse_operation`, `tool_error`, `LiveBackend`), `session/mod.rs`
(`AppDocument::edit/save/save_as/open_path/undo/redo`), `session/mcp.rs`,
`vexel-protocol/src/tools.rs` (`tool_schemas_for_preview` allow-list),
`vexel-mcp-access` (`AccessPolicy`), `vexel-v1-document` draw API
(`add_line_with_id`, `add_path_with_id`, `set_fill`, `set_stroke`, `translate`,
`delete`), ADR-0029, WALKTHROUGH-01.

## Assumptions
- `V1Document` already offers every edit needed; no document-crate change.
- The preview allow-list in `vexel-protocol` `tool_schemas_for_preview` is the
  only protocol change needed (extend the `matches!` list with the routed
  names; schemas and the 43 declared tools stay as they are).
- Grants are an out-of-band owner decision. No MCP tool may grant access (an
  agent must not self-grant); there is no `grant` tool to add.
- No PNG encoder exists in the workspace and none may be added.

## Scope
1. Library target: `vexel-app/src/lib.rs` declares `pub mod session`,
   `animation`, `tools`, `project_session`, plus the modules those need
   (`preview`, `model`, `render`, `compose`, `file_flow`, `file_ui`,
   `preview_app` as `pub(crate)` unless a test needs them public). `main.rs`
   keeps only `main`, the native `run` loop and its own tests, importing from
   `vexel_app::`. Manifest: add `[lib] name = "vexel_app" path = "src/lib.rs"`
   keeping `[[bin]] vexel`; no dependency changes. `#![forbid(unsafe_code)]` in
   both roots. Behaviour of the binary is unchanged.
2. New `Operation` variants and parsing (bounded, typed) for:
   - `save` (documentId): `AppDocument::save`; error if untitled.
   - `save_as` (documentId, path <=1023 bytes): path resolved through
     `AccessPolicy::resolve_save_path`; `.vexel` extension required.
   - `document_open` (path): resolved through `resolve_existing`; `.vexel` or
     `.svg` by extension via `AppDocument::open_path`; replaces the document,
     bumps revision, resets MCP id allocation from the opened source ids.
   - `attribute_set` (documentId, elementId, name, value): names limited to
     `fill`, `stroke`, `stroke-width`, `transform`. fill/stroke via
     `set_fill`/`set_stroke` (stroke keeps current width unless `stroke-width`
     is given; width 0..=256); `transform` accepts only
     `translate(dx dy)` or `translate(dx,dy)` (finite, |v|<=100000) via
     `translate`. Anything else is a typed error.
   - `element_delete` (documentId, elementId): `V1Document::delete`, non-root.
   - `element_add` templates `line` (x1,y1,x2,y2,stroke,stroke-width) and
     `path` (d,fill,stroke,stroke-width) via `add_line_with_id` /
     `add_path_with_id`; `d` <=4096 bytes; existing `rect`/`ellipse` unchanged;
     `circle`, `g`, `text` stay rejected as `unsupported preview shape`.
   - `export_svg` (documentId): returns `{"svg":"<source>"}` when source
     <=64 KiB, else typed error. Read-only.
   - `render_png`: DEFERRED. No PNG encoder is available; the tool stays
     unrouted and unlisted in preview and returns `unsupported preview
     operation`. `result.md` records this as a gap for a later order.
3. Every mutating tool goes through `AppDocument::edit` (one undo entry, marks
   dirty, bumps `revision` so the shared driver/strip re-render); rejected
   input leaves source bytes, history and revision untouched.
   `save`/`save_as` clear dirty only; `document_open` clears history per
   SAVE-01 semantics. Replies name the operation, `elementId` where relevant,
   and `dirty`, e.g. `{"ok":true,"operation":"attribute_set","elementId":"vexel-object-2","dirty":true}`.
4. Typed errors: extend `tool_error` allow-list with distinct strings
   (`path not granted`, `path not found`, `unsupported file type`,
   `invalid project file`, `unknown element`, `unsupported attribute`,
   `document has no path`, `export_svg too large`); each maps one
   `AccessError`/`SessionError` variant; unknown text never leaks paths.
5. Grant mechanism: `McpGrants` (new, in `session/`) holds explicit grant
   directories and builds one `AccessPolicy` via
   `AccessPolicy::new(Some(joined dirs), home)` (combines with the existing
   `~/.config/vexel/mcp-allowed-dirs.txt` and env var, never replaces them).
   Sources: repeatable CLI `--grant-dir <path>` parsed in
   `compose::parse_launch` (directory must canonicalize through
   `resolve_granted_directory`-style checks: exists, is a directory, not `/`,
   not the home directory itself) and a constructor for tests. `McpHost` and
   `ExportJob::spawn` receive this policy instead of calling
   `AccessPolicy::from_environment()` directly. Grants are fixed at launch;
   none are added at runtime.

## Non-goals
`render_png`, new protocol tools, remote transport, multi-document, element
reorder/group/duplicate, text, SMIL editing beyond `animation_add`, window or
UI changes, recovery, performance.

## Files allowed to touch
- `rust/crates/vexel-app/src/**`, `rust/crates/vexel-app/tests/**`,
  `rust/crates/vexel-app/Cargo.toml` (only the `[lib]` table; no deps)
- `rust/crates/vexel-protocol/src/tools.rs` (only the preview allow-list) and
  its existing tests
- `rust/fixtures/v1/mcp-tools/**` (new), `artifacts/WO-RUST-V1-MCP-TOOLS-01/**`

## Protected paths
Workspace/lock manifests, all other crates, C/ObjC, `tests/`, goldens, oracles,
`docs/**`, `STATE.md`, `LEDGER*.md`, `ledger.jsonl`, other work orders, git
HEAD/index. No unsafe, new external deps, network, installs, GUI launch,
commit, stage, reset, deletion. Files under 800 lines; split rather than grow
`preview.rs`/`mcp.rs` past it.

## Acceptance criteria
All tests headless (no window), driving synthetic JSON-RPC `tools/call` lines
through `vexel_mcp_stdio::serve` with `LiveBackend` on a channel served by an
`McpHost` + `AppDocument` pump in the test, or `McpHost::handle` directly.
1. `cargo build` of the `vexel` bin and the new lib both succeed;
   `tests/` integration file can `use vexel_app::session::AppDocument`.
2. `tools/list` in preview mode includes the 8 newly routed names and not
   `render_png`; the 9 existing tools behave byte-identically (existing tests
   pass unmodified).
3. Per routed tool: one success test (source bytes change exactly as specified,
   `dirty` true, `revision` +1, one undo restores prior bytes exactly, redo
   reapplies) and one rejection test per bound (oversize string/path/`d`,
   non-finite number, bad colour, unknown element, unsupported attribute or
   transform form, wrong documentId) asserting typed error plus unchanged bytes,
   history count and revision.
4. `save`/`save_as`/`document_open`: save_as into a granted temp dir then
   `document_open` in a fresh `AppDocument` yields identical `source()` bytes;
   `.svg` open works; `save` on untitled, ungranted path, missing file, wrong
   extension, corrupt and truncated `.vexel` each return their distinct typed
   error and leave the live document untouched.
5. Grants: no grant means every path tool fails `path not granted`; a
   `--grant-dir` parse test (accepted, nonexistent, file, `/`, `$HOME`
   rejected); a symlink inside the grant pointing outside is refused;
   `ExportJob` uses the same policy (export outside grant refused).
6. `export_svg` result decodes through `vexel-svg-decode` and equals
   `source()`.
7. Playback/strip still re-sync after each routed edit (revision observed by
   `Driver::knows`).
8. Existing `vexel-app`, `vexel-protocol`, `vexel-mcp-stdio` tests still pass
   (counts only increase); no new external package; no `unsafe`.
9. `result.md` (<=400 words) lists routed tools, the `render_png` deferral and
   its reason, and the native run loop left unexecuted.

If an AC cannot be met, stop with the exact blocker; no test weakening.

## Verification (frozen; repo-local CARGO_HOME/CARGO_TARGET_DIR, locked, offline)
- `cargo +stable check -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-app --locked --offline`
- `cargo +stable test -p vexel-protocol -p vexel-mcp-stdio -p vexel-v1-document --locked --offline`
- `cargo +stable fmt --manifest-path Cargo.toml --package vexel-app --package vexel-protocol -- --check`
- `python3 scripts/wo_validate.py WO-RUST-V1-MCP-TOOLS-01 --ready`
- `python3 scripts/ledger.py validate ledger.jsonl`
- `git diff --check`

## Evidence
`artifacts/WO-RUST-V1-MCP-TOOLS-01/`: `result.md`, `verification.json`,
`ac-map.json`, `changed-path-audit.json`, `tool-matrix.md` (tool, test names).

## Worker and review
Worker: gpt-6-luna High, Standard speed, one writer, existing allowance only; stop
on quota errors. Candidate only until a fresh read-only reviewer (separate
session, Luna or Sol Medium, diff + order only) reruns the frozen commands and
checks each AC. The coordinator integrates the exact patch, reruns main-tree
checks, then updates STATE.md, LEDGER.md and ledger.jsonl.
