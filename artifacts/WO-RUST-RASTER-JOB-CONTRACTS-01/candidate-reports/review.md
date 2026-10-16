# Independent Luna High review

The user-requested independent reviewer `/root/luna_review` used gpt-6-luna High
in read-only mode. It read owner/order and the current shared additions. It did
not edit files, delegate, fetch or commit.

Findings resolved during review:

1. Append cancellation originally retained a newly published capacity/lease.
   Growth now stages the whole Copy buffer/suffix and commits after final polling.
2. Replacement publication lacked a post-copy control check. Copy loops and final
   publication checkpoint are now inside the transaction.
3. Wrong pool handles could bypass local caps/output tags. Buffer element roles,
   dash/grid/scratch/output consumption and release now validate pool roles.
4. Duplicate scratch handles could partially release. Shared uniqueness preflight
   runs before both validation and release.
5. Actual reported capacity lacked a per-buffer cap check. It is now checked
   before copy/publication alongside global/pool/grid bounds.
6. Scratch early release lacked role checks for forged same-job fields. The
   fixed-field role preflight is now shared with validation, with a regression.
7. Failed initial ledger transfer now returns the unchanged ledger for recovery.

Final reviewer response: “All previously reported findings are resolved in the
current files ... I found no remaining concrete blocker in this read-only review.”

The reviewer explicitly distinguishes active requested-byte admission before
allocation from post-return validation of any Vec-reported excess. Safe std does
not expose allocator capacity before returning it. No allocator/RSS guarantee or
global OOM fault injection is claimed. The coordinator's separate fresh-main
integration/protected audit is still required before acceptance/peer seed.

Final boundary-source correction: original 200M is marking work per mask, not
bits. A standalone copyable counter was rejected in review. The final counter
is private in each live Boundary buffer slot; copied handles share it and
clear_capacity cannot reset it. charge_boundary stages job work stats and commits
only after actual control succeeds. The reviewer confirmed alias/overflow/clear/
cancel coverage and reported no remaining blocker in the final sanity check.
