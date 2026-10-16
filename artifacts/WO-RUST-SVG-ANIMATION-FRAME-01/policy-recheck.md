## ACCEPT — owned frame/clip source only

The revised candidate clears the R2 blockers. `evaluate_frame` requires an explicit finite caller budget and passes it into frame construction; finished-frame clip replacement accounts for old/new overlap. The 64 MiB limit remains a separate clip-table cap. The recorded tests demonstrate overlap above 64 MiB under a 128 MiB caller policy and atomic rejection of a 65 MiB clip.

I reviewed the 31 mapped symbols against both original C files and the Rust implementation. The transform, geometry, paint-inverse, motion, clip-owner, ancestor-chain, and overlay-order algorithms are present. Clip replacement and frame construction publish only after successful validation; failures leave the base and existing frame data unchanged. The loop audit and tests cover in-loop cancellation, including the zero-node/zero-edge suffix, while preserving the original local work accounting.

The reports account for all 52 original cases: pure animation assertions are represented by Rust scene/overlay tests, mixed renderer cases remain partial or deferred for real renderer integration, and authoring, CLI, and fuzz cases are explicitly deferred to their owning integrations. I found no silent omission or fake-success path in this scope. Source and test files are reported below 800 lines.

The earlier rejected notes record the missing-fixture/API state and the obsolete 64 MiB compatibility default. The current source uses the explicit-budget API and includes the R2 proofs.

Recorded verification reports exit 0 for the locked offline package check and test (31 passed), rustfmt, and R2 work-order validation. I assessed this evidence without rerunning commands.

Remaining integration gates are the real DecodeSession/FlatKernel/native renderer pixel oracles, animation CLI and sequence limits, source-preserving authoring/editor tests, and the Rust decoder fuzz campaign. This accepts the owned source candidate only; it does not claim full product integration.