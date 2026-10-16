**ACCEPT.** The previous gate had one blocker: ignored notices survived draft rollback. The new snapshot and restore close that gap.

`begin_draft` captures the ignored array; rollback validates the active top token before changing builder state, then restores the snapshot only after scene rollback succeeds. Token checks bind authorization to scene identity, nonce, and scene count, rejecting foreign, stale, and non-top tokens. Nested snapshots give the requested behavior: child rollback restores parent notices, child commit retains its notices for the parent, and parent rollback removes both. Ordinary marks still do not rewind ignored notices.

The two added tests cover count and first-offset restoration, nested rollback and commit behavior, and notices retained after a successful root commit. The README describes the distinction between ordinary marks and draft rollback.

The prior blocker is resolved. I reviewed the reported result of all 57 focused tests; I did not run commands. This accepts only the scoped scene-draft gate, with no claim about native, renderer, or application completion.