# Security findings and disposition

1. The executor uses the owner-approved `--dangerously-bypass-approvals-and-sandbox` invocation. This is an explicit BOOT-04 exception recorded in `docs/adr/0003-codex-cli-compatibility-amendment.md`; executor cwd is the task worktree, and candidate paths are checked against frozen allowlists before acceptance.
2. Frozen work-order content is read from the validated repository copy. Candidate changes to the active work-order file are rejected unless the candidate contains byte-identical content. Verifier mode independently checks candidate scope before review.

Both findings are dispositioned for BOOT-04. The CLI bypass remains an accepted workflow limitation under the owner directive.
