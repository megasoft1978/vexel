# WO-RUST-SVG-ANIMATION-FRAME-01 R2 result

Status: owned candidate prepared for independent review; acceptance pending.

The normal `evaluate_frame` API now requires a caller-configured finite frame byte budget. It passes that policy unchanged to `FrameBuilder`; finished-frame clip replacement retains it for actual old/new capacity overlap. No whole-frame default exists. The original separate 64 MiB clip-copy cap remains.

All owned variable loops were audited. Edge mapping/bounds now poll each edge; clip bounds poll every shape, including long zero-node/zero-edge suffixes. Reverse transform products, initialization, overlay detection, inactive items, clip flags, color edges and publication comparisons also poll actual iterations. Original local work accounting is separate: aggregate edge totals, original target visit math, 64 units per shape for clip-owner update, 50,000,000 cap and saturated boundary behavior remain.

31 tests pass. The prior27 predicates/data remain, except the explicitly incorrect whole-frame default assertion was replaced with stronger policy proof. Actual nested33 MiB clips successfully overlap above64 MiB under caller128 MiB policy; a single65 MiB clip still rejects atomically. Real scenes with1024 empty suffix shapes prove cancellation during bounds traversal, unchanged base/frame/stats and successful uncancelled updates. Edge tests preserve local work while stopping mid-loop.

Scoped locked offline check/test, rustfmt and frozen R2 validation pass. All source/test files are below800 lines. Reports retain all31 symbols and52 original cases, with original renderer/CLI/authoring/fuzz gates still marked later. Immutable scope/seeds and protected shared/manifests/ledgers were not edited. Independent review and coordinator acceptance remain required.
