# WO-RUST-SVG-DECODE-01 R2 — continuation 6 candidate

Status: **incomplete candidate; do not integrate or accept.**

Verified all four Scene files in `accepted-observability-source-seed.json` against their accepted-main SHA-256 values. Added a decoder ledger-history accessor backed directly by `ChargeLedger::allocation_history()`. The frozen 512 late-gradient input now traverses the real Decoder; the test checks the accepted `DecodeSource::reference_lookup_probes()` bound and exact decoder ReferenceIndex allocation-history bytes against `reference_storage_bytes()`. A direct Decoder test also checks opacity-group shape range and context publication. All 113 catalog keys and continuation-5 tests remain.

The locked offline decoder check, full package tests (88 passed across 27 result blocks), scoped rustfmt check, and ready-order validator passed. Source/test files remain below 800 lines (max source 792; max test 616). Exact commands and logs are in `verification.json`.

Still open: production paint-token visits; exact three-input allocation deltas; accepted definitions-backed clip source-span/capacity and resolved-once history; real renderer output accounting of 256 bytes; tile-cache predicates; whole renderer/CLI/oracle evidence; independent review and coordinator integration. Existing fixture-backed tests remain fixture-scoped. No R2 completion or acceptance claim is made.
