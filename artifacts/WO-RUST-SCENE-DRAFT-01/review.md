**BLOCKER — `record_ignored` bypasses draft scope control.**

[builder.rs](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-scene/src/builder.rs:300) lets `record_ignored` mutate builder state while a draft is active. That state is outside the scene checkpoint: [rollback_draft](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-scene/src/draft.rs:96) truncates scene tables and descendants but does not restore `ignored`.

A caller can begin a draft, call `record_ignored`, then roll back; the ignored count and offset remain and can be published by a later freeze. This also allows mutation while a nested child draft is pending, bypassing the parent-mutation guard.

Guard `record_ignored` during active drafts, or include its state in draft checkpoints and rollback. Add coverage for rollback and pending-child behavior. I did not run tests or builds, per instruction.