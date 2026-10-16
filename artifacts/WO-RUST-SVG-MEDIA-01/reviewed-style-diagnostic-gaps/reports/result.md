# WO-RUST-SVG-MEDIA-01 R2 — Scene153 candidate

Implemented request atomicity using the accepted public begin_request/commit_request/rollback_request APIs with the caller’s real DraftToken. Image and text operation or commit failures restore the scene suffix, preserve prefix handles and leave the outer draft usable. Text errors also restore every external TextState field, cache admissions and counters while retaining earlier owned layouts. Capacity/allocation/work history may remain; no RSS rollback is claimed.

Every validated layout use, including cache hits, now calls accepted SceneBuilder::admit_text_commands at the original run offset. Scene153 intentionally holds this cumulative budget on the builder and leaves TextState unchanged. Sixteen cached 262144-command rectangle layouts reach exactly 4 Mi; use seventeen fails with TextCommands, preserving earlier output and counters. A late canceled cached use restores its nonzero checkpoint. Cache admission limits remain independent.

All eight previous defect demonstrations now require successful cleanup/limit behavior, documented in [assertion-transition.md](assertion-transition.md). All 22 prior functional tests remain active. Additional tests sweep every observed control boundary through request commit for cancellation, work limits and arithmetic stops, and exercise late provider errors/malformed output. Original provider failures preserve Limit/OOM/otherwise Unsupported classifications and source offsets. Corrected style priority, font/tspan/spacing/anchor/cache proofs and compound contours remain intact.

All 24 catalog symbols and 12 original cases remain mapped in [parity-map.json](parity-map.json) and [coverage.json](coverage.json). Protected seeds and original inputs are unchanged. Only frozen scoped offline check/test/WO validations were run; exact evidence is in [verification.json](verification.json).

Remaining pure verification: actual global allocator OOM injection is unexecuted because no accepted safe injection hook exists; real capacity-admission and provider OOM failures are tested. Fuzz release ownership is adapted to Rust Drop; no fuzz campaign is claimed. Native providers, full mixed renderer/CLI/C/selection oracles, CoreText/librsvg and application/runtime/release/UI proof remain later mandatory gates.

Candidate only: independent review, protected audit and fresh main checks are still required before acceptance.
