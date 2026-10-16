Implemented all five frozen R1 shared operations in the isolated scene writer:

- Request/draft-owned cumulative 4 Mi document text-command admission, separate
  from the unchanged 262144-command cache policy and TextState literals. Successful
  cached uses count; provider/publication failure, abort and nested rollback restore
  logical use while preserving real allocation/work history.
- Safe same-job composition with one validated immutable dash/grid scope, checked
  disjoint admitted scratch/fill/row/output writes, same ledger/control and no
  cache/source clone, unsafe, self-reference or per-row allocation. Legacy controlled
  views retain explicit validation. Errors/panic preserve honest unfinished buffers.
- Canonical original post-Z overlap acceptance at append and sealed validation,
  backed by original segment/source/closure metadata; nothing rewritten or dropped.
- Actual source/draft/request/group/range mask-post targets, production plan checking
  and atomic attachment; no guessed group or fake shape.
- Explicit bounded atomic soft-quantum semantics, all charge reports and overshoot
  visible, original caps intact, pending owned input returned at boundaries.

The sole new public fixture file exercises actual lexical/path/animation/provider/
planner/builder/frame/job seams, including exact limits, failure, cancel, alias,
stale scope/serial, arithmetic, native capacity rejection, physical history and
metadata preservation. APIcoverage.md maps every added operation to evidence.
README specifies ownership, cleanup, exact atomic scope and lower adapter obligations.

Verification: required offline check and full scene tests pass; frozen ready validator
passes revision 1 / ec38f7a24f42db58a2d2cff1424c3add6f16a7fa083cf8bd6eb69cf4a37cb181.
151 named tests pass (135 accepted unchanged + 16 new), plus two compiled negative
ownership checks. Exact commands, exits and logs are in verification.json.

Preservation audit: 2876 protected files unchanged; supplied 1709-file seed has no
protected mismatch. Existing source test blocks/files are byte equal to current main;
old test files/assertions/literals remain intact. HEAD matches seed and index matches
HEAD. All scene sources and the new test remain below 800 lines. No peer source,
manifest, lock, root ledger, order, source/oracle fixture, staging or commit changed.

Independent coordinator review/fresh-main acceptance and peer seeding remain pending.
No protected dependency blocks these shared operations. Lower engine adaptations,
original decoder assertion rerun after accepted seeding, full renderer/native/root
integration, 603/157 oracle execution, large-file measurements and final application
acceptance remain downstream. No global OOM/RSS/performance/fidelity/final-QA claim.
