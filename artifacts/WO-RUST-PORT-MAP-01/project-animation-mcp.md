# Project, animation, jobs, and MCP crosswalk

Scope: frozen baseline paths in the assigned `src/project/**`, `src/jobs/**`, `src/mcp/**`, and `tests/project|jobs|anim|mcp/**` areas. Inventory dispositions remain `pending_port`; these are candidate mappings only, not parity or migration closure. Source and acceptance results were inspected; no builds, tests, benchmarks, or GUI checks were run.

## Rust candidate modules and accepted evidence

| Legacy behavior | Rust candidate and frozen evidence | Assessment |
|---|---|---|
| Project bundle/package serialization, file IO, assets, and recovery | `rust/crates/vexel-project/src/{bundle.rs,lib.rs}`, `rust/crates/vexel-project-io/src/{file.rs,file_load.rs,recovery.rs,recovery_discovery.rs}`, `rust/crates/vexel-app/src/project_session/{mod.rs,payload.rs,actions.rs,recovery.rs,recovery_store.rs}`. Accepted WO-RUST-PROJECT-PAYLOAD-01 covers opaque manifest/timeline/assets bytes through open/edit/save/save-as/reopen and failure atomicity; WO-RUST-V1-INTEGRATE-01 records integrated project IO. | Partial: project payload and IO have concrete candidates, but legacy bundle overlay/drawing-asset behavior and full frozen contract are not demonstrated by those results. |
| Timeline model, JSON, edits, sessions, and native integration | `rust/crates/vexel-timeline/src/{schema.rs,json.rs,access.rs,edit.rs,edit/ops.rs}`; app presentation/model in `rust/crates/vexel-app/src/timeline/**`. Frozen Rust acceptance includes `vexel-timeline/tests/{parity.rs,edit.rs}` and app timeline tests; WO-RUST-UI-TIMELINE-01/02 report timeline interface work. | Partial: core timeline schema/edit mappings exist; native timeline integration, sessions, exact legacy edit edge cases and serialization parity still need assertion-by-assertion mapping. |
| SVG animation evaluation/render/export | `rust/crates/vexel-svg-animation/src/{parse.rs,eval.rs,control.rs,motion.rs}`, `rust/crates/vexel-svg-animation-frame/src/{apply.rs,clips.rs}`, plus `rust/crates/vexel-app/src/animation/{frame.rs,export.rs,job.rs,playback.rs,present.rs}`. WO-RUST-V1-ANIM-01 records GIF export and animation candidate evidence; accepted Rust animation tests are in `vexel-svg-animation/tests/animation.rs` and app `animation/tests*.rs`. | Partial: general SVG animation and app export candidates exist. C ImageIO animation-render semantics, authoring operations, CLI/fuzz behavior, exact frame/color/delay/size limits and live playback wiring are not established as equivalent. |
| Job scheduling and immutable source snapshots | Rust queue candidate: `rust/crates/vexel-platform-tasks/src/queue.rs::{TaskQueue,TaskCallback,CancellationToken,TaskResult}`; exact accepted evidence is WO-RUST-TASK-QUEUE-01 (`ledger.jsonl`, accepted event `WO-RUST-TASK-QUEUE-01-r1-accepted-integrated`): integrated focused package verification reports 11 tests passing, including named tests `configuration_and_fallible_bounded_allocations`, `worker_start_failure_stops_and_joins_started_workers`, `status_priority_duplicate_full_stale_and_stopped`, `rejected_submission_returns_callback_and_context`, `queued_cancel_drops_owned_context_once_outside_queue_lock`, `running_cancel_and_generation_advance_suppress_publication`, `cancellation_after_running_transition_still_invokes_callback_without_publication`, `callback_failure_and_panic_are_retired_as_results`, `completion_releases_id_only_after_poll`, and shutdown/cleanup race tests. Rust document snapshot candidate is `rust/crates/vexel-v1-document/src/lib.rs::V1Document::edit_snapshot` returning `EditSnapshot`; named Rust oracle `edit_snapshot_shares_an_immutable_view_of_source_bytes` is core-owned and only cross-referenced here. Historical frozen C acceptance WO-M3-03 (`ledger.jsonl`, accepted event `m303-r1-accepted`) reports 16 frozen commands passed, CTest 10/10 in Debug/ASan/UBSan, TSan 1/1, with `artifacts/WO-M3-03/snapshot-tests.txt` and `scheduler-tests.json` as named evidence. | Partial for Rust task queue only. The queue result does not implement or establish source-byte snapshot parity; `V1Document::edit_snapshot` is a core document snapshot candidate, not a replacement for jobs `source_snapshot`. Keep those source rows unresolved pending dedicated jobs snapshot mapping. |
| MCP JSON-RPC, tool schemas, access, broker, export, task store | `rust/crates/vexel-protocol/src/{protocol.rs,async_protocol.rs,tools.rs,protocol/tool_methods.rs}`, `rust/crates/vexel-app/src/session/{mcp.rs,mcp_tools.rs,mcp_png.rs,grants.rs}`, `rust/crates/vexel-mcp-tasks/src/lib.rs`, and project/app export paths. WO-RUST-V1-MCP-TOOLS-01 reports live routing for save/save_as/document_open/edit/export_svg and preview `tools/list`; WO-RUST-MCP-ACCESS-01 and WO-RUST-MCP-TASK-STORE-01 provide access/task-store candidates; WO-RUST-MCP-STDIO-01 covers transport; WO-RUST-RECOVERY-MCP-ERROR-01 records stable recovery error mapping. | Partial: protocol, selected tools, grants, async tasks and export have candidates and accepted work-order evidence. Legacy broker tool coverage, native timeline, render/preview semantics, complete transport/socket lifecycle and full frozen protocol/fuzz/e2e oracle remain unresolved. |

## Frozen source rows

Each row below retains its baseline `pending_port` disposition. “Partial” identifies concrete Rust candidates above, not proven replacement.

| Baseline path | Classification / mapped behavior |
|---|---|
| `src/jobs/job_scheduler.c` | Partial — `vexel-platform-tasks::queue::{TaskQueue::submit,cancel,advance_generation,poll,shutdown}`; WO-RUST-TASK-QUEUE-01 accepted integrated result (`ledger.jsonl`, 11 package tests) covers bounded queue, cancellation, generation filtering, polling, shutdown and cleanup. This scoped mapping does not prove the full legacy scheduler worker/result contract identical. |
| `src/jobs/job_scheduler.h` | Partial — `queue.rs::{QueueConfig,QueueError,SubmitError,TaskResult,TaskQueue}` with WO-RUST-TASK-QUEUE-01 accepted integrated result; typed Rust ownership/callback API is a candidate, while exact legacy interface/status mapping still needs comparison. |
| `src/jobs/source_snapshot.c` | Unresolved — core-owned `V1Document::edit_snapshot`/`EditSnapshot` is an adjacent immutable document-edit snapshot, with named test `edit_snapshot_shares_an_immutable_view_of_source_bytes`; no evidence establishes equivalence to the legacy jobs source-byte snapshot API, limits or lifetime. Do not duplicate core snapshot storage. |
| `src/jobs/source_snapshot.h` | Unresolved — cross-reference only: core-owned `V1Document::edit_snapshot -> EditSnapshot`; equivalence to this jobs interface is unestablished. Keep jobs interface analysis separate from core document snapshot ownership. |
| `src/mcp/mcp_access.c` | Partial — `vexel-app::session::grants` / access policy (WO-RUST-MCP-ACCESS-01); path canonicalization and all legacy access edge cases unresolved. |
| `src/mcp/mcp_broker.c` | Partial — `vexel-app::session::mcp::McpHost` and session tool routing (WO-RUST-V1-MCP-TOOLS-01); complete broker lifecycle and dispatch parity unresolved. |
| `src/mcp/mcp_broker.h` | Partial — `McpHost` candidate; interface behavior beyond accepted routed tools unresolved. |
| `src/mcp/mcp_broker_animation.c` | Partial — app animation/export and MCP animation tool routes; legacy broker animation/render tool behavior unresolved. |
| `src/mcp/mcp_broker_docs.c` | Partial — `mcp_tools` document open/save and `vexel-project-io`; complete docs operations and error behavior unresolved. |
| `src/mcp/mcp_broker_edit.c` | Partial — `mcp_tools` edit methods; accepted V1 work covers selected attributes, add line/path, and delete, not every legacy edit. |
| `src/mcp/mcp_broker_internal.h` | Partial — private host/backend structures in `session/mcp.rs`; private contract parity unresolved. |
| `src/mcp/mcp_broker_native_timeline.c` | Unresolved — `vexel-timeline` and app timeline model/edit behavior do not establish an MCP backend route; no accepted same-behavior MCP native-timeline implementation or route evidence is identified. |
| `src/mcp/mcp_broker_render.c` | Partial — render/export candidates exist; V1 MCP evidence explicitly deferred `render_png` then, and does not establish full broker render parity. |
| `src/mcp/mcp_broker_tools.c` | Partial — `vexel-protocol::tools` plus app backend; only accepted preview tool subset established by WO-RUST-V1-MCP-TOOLS-01. |
| `src/mcp/mcp_export_job.c` | Partial — app animation job/export and project export paths; MCP export job lifecycle parity unresolved. |
| `src/mcp/mcp_export_job.h` | Partial — same job candidate, interface details unresolved. |
| `src/mcp/mcp_export_snapshot.c` | Unresolved — project session payload storage does not establish an immutable MCP export snapshot implementation; snapshot ownership, immutability and lifetime lack concrete same-behavior Rust symbols/tests and accepted evidence. |
| `src/mcp/mcp_export_snapshot.h` | Unresolved — no concrete Rust MCP export-snapshot interface or accepted ownership/lifetime contract identified; project payload storage is not evidence for this interface. |
| `src/mcp/mcp_json.c` | Partial — `vexel-protocol` JSON-RPC/protocol decoding; exact JSON value limits and error behavior need mapping. |
| `src/mcp/mcp_json.h` | Partial — protocol types candidate; full JSON API parity unresolved. |
| `src/mcp/mcp_protocol.c` | Partial — `vexel-protocol::protocol` and `tools`; protocol tests exist, but frozen legacy assertions not yet crosswalked. |
| `src/mcp/mcp_protocol.h` | Partial — same protocol candidate. |
| `src/mcp/mcp_protocol_async.c` | Partial — `vexel-protocol::async_protocol` and `vexel-mcp-tasks`; task/protocol cancellation and ordering equivalence unresolved. |
| `src/mcp/mcp_protocol_async.h` | Partial — same async protocol candidate. |
| `src/mcp/mcp_socket.c` | No Rust candidate found — transport stdio work does not evidence legacy socket behavior. |
| `src/mcp/mcp_socket.h` | No Rust candidate found — no socket interface candidate. |
| `src/mcp/mcp_task_store.c` | Partial — `vexel-mcp-tasks`; WO-RUST-MCP-TASK-STORE-01 evidence needs row-level comparison for retention, result/error and task lifetime. |
| `src/mcp/mcp_task_store.h` | Partial — same task-store candidate. |
| `src/mcp/mcp_tools.c` | Partial — protocol tool schema and app backend; accepted subset is documented in WO-RUST-V1-MCP-TOOLS-01. |
| `src/project/project_animation_render.c` | Partial — SVG animation evaluation/frame rendering candidates; ImageIO render semantics and limits not proven by WO-RUST-V1-ANIM-01. |
| `src/project/project_animation_render.h` | Partial — same candidate; public contract parity unresolved. |
| `src/project/project_bundle.c` | Partial — `vexel-project::bundle` and `vexel-project-io`; payload retention accepted in WO-RUST-PROJECT-PAYLOAD-01, full bundle semantics unresolved. |
| `src/project/project_bundle.h` | Partial — same bundle candidate. |
| `src/project/project_bundle_asset_overlay.c` | Partial — `vexel-project::bundle` asset storage candidate; overlay precedence/merge behavior lacks accepted evidence. |
| `src/project/project_bundle_asset_overlay.h` | Partial — same candidate; contract unresolved. |
| `src/project/project_drawing_asset.c` | Partial — project bundle asset payload candidate; drawing asset decode/ownership behavior unresolved. |
| `src/project/project_drawing_asset.h` | Partial — same candidate. |
| `src/project/project_file.c` | Candidate replacement — `vexel-project-io::file` and app project session; accepted payload and integration evidence supports basic open/save, but exact frozen test claims remain to be mapped. |
| `src/project/project_file.h` | Partial — Rust file API candidate; public error/atomicity contract not fully crosswalked. |
| `src/project/project_recovery.c` | Partial — `vexel-project-io::recovery` plus app `project_session::recovery`; accepted recovery work orders cover selected flows, not all legacy invariants. |
| `src/project/project_recovery.h` | Partial — same recovery candidate. |
| `src/project/project_timeline.c` | Partial — `vexel-timeline::schema/access` and app timeline model; baseline timeline ownership/validation details need exact mapping. |
| `src/project/project_timeline.h` | Partial — same candidate. |
| `src/project/project_timeline_edit.c` | Partial — `vexel-timeline::edit`; frozen command semantics and all edge cases unresolved. |
| `src/project/project_timeline_edit.h` | Partial — same edit candidate. |
| `src/project/project_timeline_integration.c` | Partial — app timeline UI/model candidate; integration with document/project state not established against all frozen assertions. |
| `src/project/project_timeline_integration.h` | Partial — same integration candidate. |
| `src/project/project_timeline_json.c` | Partial — `vexel-timeline::json`; `tests/parity.rs` is relevant accepted Rust oracle, but full C assertion correspondence is not documented. |
| `src/project/project_timeline_json.h` | Partial — same JSON candidate. |
| `src/project/project_timeline_loop_edit.inc` | Partial — `vexel-timeline::edit::ops`; loop edit-specific endpoint/wrap semantics need mapping. |
| `src/project/project_timeline_move_edit.inc` | Partial — `vexel-timeline::edit::ops`; move collision/clamp/selection semantics need mapping. |
| `src/project/project_timeline_property_edit.inc` | Partial — `vexel-timeline::edit::ops`; per-property validation and atomicity need mapping. |
| `src/project/project_timeline_session.c` | Partial — app `timeline` state and project session; session history/persistence semantics need evidence. |
| `src/project/project_timeline_session.h` | Partial — same session candidate. |

## Frozen test and oracle rows

These are preserved frozen test paths. Rust test names below are candidate evidence, not proof that each frozen assertion is covered.

| Frozen test path | Classification / candidate oracle |
|---|---|
| `tests/anim/__init__.py` | Unresolved — Python package marker has no semantic assertion; retain as frozen tooling input pending its suite disposition. |
| `tests/anim/anim_test.c` | Unresolved — general Rust animation evaluation tests do not establish this frozen authoring/evaluation oracle; concrete same-behavior symbols/tests and accepted assertion correspondence are missing. |
| `tests/anim/animation_authoring_edge_test.c` | Unresolved — general app/SVG animation tests do not establish authoring edge-case behavior; no concrete same-behavior Rust authoring symbols/tests with accepted evidence identified. |
| `tests/anim/animation_authoring_test.c` | Unresolved — general SVG animation evaluation and app export tests do not establish authoring operations; concrete same-behavior Rust authoring symbols/tests and accepted oracle correspondence are missing. |
| `tests/anim/animation_eval_cli.c` | No Rust candidate found — no equivalent frozen CLI candidate identified. |
| `tests/anim/animation_fuzz.c` | Unresolved — general Rust animation parser/evaluator tests do not establish the frozen fuzz harness/corpus contract; no concrete same-behavior Rust fuzz oracle with accepted evidence identified. |
| `tests/anim/animation_render_cli.c` | No Rust candidate found — no equivalent frozen render CLI identified. |
| `tests/anim/test_animation.py` | Unresolved — general Rust animation tests do not establish the frozen Python harness, CLI invocation or output assertions; no accepted same-behavior harness evidence identified. |
| `tests/anim/test_render_reference.py` | Unresolved — general Rust frame/export tests do not establish the frozen Python reference-image harness or oracle; no accepted same-behavior reference comparison evidence identified. |
| `tests/jobs/test_job_snapshot.c` | Partial — frozen oracle covers snapshot byte ownership/mutation/limits, concurrent retain/read/release, deterministic digest, capacity/backpressure, and generation/cancel cleanup (WO-M3-03 `snapshot-tests.txt` and `scheduler-tests.json`; accepted historical C run recorded as 16 commands, CTest 10/10 Debug/ASan/UBSan, TSan 1/1). Rust `vexel-platform-tasks/src/tests.rs` has named queue/cancel/cleanup tests but no source-byte snapshot tests; core-owned `edit_snapshot_shares_an_immutable_view_of_source_bytes` is a cross-reference, not jobs coverage. |
| `tests/test_jobs.py` | Unresolved — frozen Python harness oracle (baseline blob `6ee1c38b0fcae8519aa5ff00b3de4aff4df402cb`) runs `build/mac-debug/bin/job_snapshot_test`; requires status `pass`, `owned_cleanup_cases == 4`, failed-submit cleanup `0`, and passing digest, byte-budget, scheduler-capacity, random-cleanup, generation-filter and random-schedule fields; it writes `scheduler-tests.json` and `snapshot-tests.txt`. Native harness/build integration is tooling-owned; this row appears here as the assigned oracle only. Rust queue evidence does not establish this Python harness/output contract. |
| `tests/mcp/__init__.py` | Unresolved — package marker; preserve with frozen test inventory. |
| `tests/mcp/mcp_async_task_test.c` | Partial — `vexel-mcp-tasks/src/tests.rs`; exact async task oracle correspondence unresolved. |
| `tests/mcp/mcp_broker_io_test.c` | Partial — app MCP tests and `vexel-protocol/tests/mcp_preview_profile.rs`; full IO broker matrix unresolved. |
| `tests/mcp/mcp_export_job_test.c` | Partial — app animation job/export tests; MCP-specific lifecycle/error/cleanup oracle not mapped. |
| `tests/mcp/mcp_export_snapshot_test.c` | Unresolved — project payload persistence tests do not establish immutable MCP export snapshots; concrete same-behavior Rust snapshot symbols/tests and accepted ownership/lifetime oracle evidence are missing. |
| `tests/mcp/mcp_features_test.c` | Partial — `vexel-protocol/tests/mcp_preview_profile.rs` and app MCP tests cover a subset of tools. |
| `tests/mcp/mcp_fuzz.c` | Partial — Rust JSON-RPC fuzz/property candidate `vexel-protocol/tests/fuzz_jsonrpc.rs`; equivalent corpus/accepted result not established. |
| `tests/mcp/mcp_headless_broker.c` | Partial — app MCP session tests; headless broker behavior matrix unresolved. |
| `tests/mcp/mcp_native_timeline_test.c` | Unresolved — Rust timeline model/edit tests do not exercise an MCP native-timeline backend route; no concrete same-behavior route test with accepted evidence identified. |
| `tests/mcp/mcp_protocol_async_test.c` | Partial — `vexel-protocol` async protocol and MCP task tests; ordering/cancel/error assertions need row-level mapping. |
| `tests/mcp/mcp_protocol_test.c` | Partial — `vexel-protocol` parity and protocol tests; complete legacy message/error cases unresolved. |
| `tests/mcp/test_e2e.py` | No Rust candidate found — broad MCP e2e oracle deferred; no equivalent accepted result identified. |
| `tests/project/project_animation_render_test.c` | Partial — SVG animation and app export tests; rendered pixels, frame schedule and failure behavior not crosswalked. |
| `tests/project/project_bundle_test.c` | Partial — `vexel-project/tests/bundle.rs` plus payload evidence WO-RUST-PROJECT-PAYLOAD-01; asset overlay and all bundle invariants unresolved. |
| `tests/project/project_file_test.c` | Partial — project IO tests and WO-RUST-PROJECT-PAYLOAD-01; exact frozen assertions not all linked. |
| `tests/project/project_load_test.c` | Partial — `vexel-project-io` load tests and payload work order; failure atomicity is evidenced for selected cases only. |
| `tests/project/project_recovery_test.c` | Partial — `vexel-project-io` recovery tests and accepted recovery work orders; full state/recovery matrix unresolved. |
| `tests/project/project_timeline_edit_test.c` | Partial — `vexel-timeline/tests/edit.rs`; no per-assertion correspondence. |
| `tests/project/project_timeline_integration_test.c` | Partial — app timeline tests and WO-RUST-UI-TIMELINE-01/02; document/session integration coverage unresolved. |
| `tests/project/project_timeline_move_test.c` | Partial — `vexel-timeline/tests/edit.rs`; move-specific constraints need direct oracle mapping. |
| `tests/project/project_timeline_session_test.c` | Partial — app timeline/project session tests; persistence/history/undo correspondence unresolved. |
| `tests/project/project_timeline_test.c` | Partial — `vexel-timeline/tests/{parity.rs,edit.rs}`; complete frozen schema, validation and serialization oracle unresolved. |

## Disjoint follow-up work packages and dependencies

1. **Project bundle/assets** — complete bundle and overlay/drawing-asset semantics. Depends on stable project payload model (`vexel-project`, payload work order); then crosswalk bundle/load/file tests.
2. **Timeline parity and session integration** — map each legacy timeline/edit/session assertion to `vexel-timeline` and app session behavior; implement uncovered cases. Depends on project payload/session interfaces and timeline edit model.
3. **Animation render/authoring** — map authoring edge cases, project frame rendering, timing, alpha/palette/limits and reference goldens. Depends on SVG animation evaluation plus render contract and project timeline clock; keep GIF export claims limited to WO-RUST-V1-ANIM-01 scope.
4. **Jobs source snapshots** — compare legacy `source_snapshot` byte-copy/limit/retain-release semantics with core-owned `V1Document::edit_snapshot` and determine whether a jobs adapter is required. Do not duplicate core snapshot storage. Then map unrepresented jobs snapshot assertions. Queue scheduling is already an accepted Rust candidate under WO-RUST-TASK-QUEUE-01; follow-up should address concrete scheduler gaps only after row-level comparison. Frozen Python harness runner/output integration stays with tooling ownership; this slice maps its oracle only.
5. **MCP transport/protocol/broker** — map full JSON-RPC, async task, socket/stdio lifecycle and tool error behavior. Depends on task-store and access policy contracts.
6. **MCP tool surface: docs/edit/render/timeline/export** — cover remaining broker operations and establish each tool's behavioral oracle. Depends on protocol/broker and the corresponding project, timeline, animation and render packages.

## Unresolved groups

- Full legacy MCP broker and tool inventory, notably native timeline, rendering, socket transport, source/task snapshots, and e2e/fuzz correspondence.
- Exact project bundle asset overlay and drawing-asset semantics.
- Frozen timeline edit/session assertions not represented by named Rust parity oracles.
- Legacy jobs source-snapshot bytes/ownership/limits are not established by accepted Rust task-queue evidence; adjacent core `EditSnapshot` is not claimed as a jobs replacement.
- Animation authoring/render reference equivalence, including exact ImageIO output and CLI/fuzz oracle relation.
- Per-test-row mapping for all C/Python frozen tests; accepted Rust package tests and work-order results currently establish only selected behavior.
