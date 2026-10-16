Repaired the viewport provenance gap. Normal initialization now parses canonical root attributes internally; callers cannot supply fabricated dimensions. Hidden roots still skip viewport parsing.

All named checks passed: **177 Scene + 21 animation checks**. Existing accepted tests remain unchanged; prior ownership, accounting, frame, and request proofs remain covered.

[Exact result, logs, and API coverage](/Users/megasoft78/Desktop/Freelance/vexel/.cache/WO-RUST-RENDER-INTEGRATION-CONTRACTS-01/worker/artifacts/WO-RUST-SCENE-DECODE-INITIALIZATION-01/viewport-repair-result.md)

Root independent re-review, protected audit, and fresh-main verification remain required before acceptance.