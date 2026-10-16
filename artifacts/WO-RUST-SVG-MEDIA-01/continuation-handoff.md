# Continuation handoff — WO-RUST-SVG-MEDIA-01

Checkpoint: 2026-10-15. The full application goal remains active and unchanged.
This is a usage-settings checkpoint, not a pause or completion. No UI/E2E tests
were run. No media worker is running.

## Next bounded action

Resume the frozen revision 2 media work order at
`work/WO-RUST-SVG-MEDIA-01.md`. The terminal candidate at
`.cache/WO-RUST-SVG-MEDIA-01/worker` preserves 54 passing focused tests and its
evidence. Its only known shared blocker was Scene pre-geometry shape admission;
that API has since been accepted in the main tree (`rust/crates/vexel-scene`;
see `artifacts/WO-RUST-SHAPE-PUBLICATION-01/final-review.md`).

Before launching a worker, archive the candidate workspace's current
`rust/crates/vexel-scene` under its own WO artifacts, seed the exact accepted
main Scene crate into the isolated media workspace, and record hashes. Preserve
all existing media candidate files. Then resume only the media crate using the
accepted `SceneBuilder::admit_shape` / `append_admitted_shape` API so shape
capacity is admitted before text/image provider work and consumed during real
publication. Keep the complete 24-symbol/12-original-case scope, the corrected
text visibility/error-order behavior, atomicity and rollback guarantees. Do not
edit shared interfaces/manifests or run UI/E2E at this milestone.

## Execution and acceptance limits

- Current project usage policy: Luna Medium, Standard speed; Sol Medium only
  for a named hard decision/repeated failure/final review.
- At most two workers/processes for this project including coordinator; one
  focused worker is sufficient. No nested workers.
- Existing subscription allowance only; no purchased credits. Stop on quota or
  rate errors without retry loops.
- The resumed candidate still requires independent review, protected-scope
  audit, and fresh named offline checks before acceptance. Do not claim the
  application complete; full renderer, UI, MCP, demo and release gates remain.

Last observed worker process status is recorded in
`artifacts/WO-RUST-SVG-MEDIA-01/process-status.json` (completed, exit 0). The
unrelated active Codex process observed belonged to `video-surveillance`, not
this project.
