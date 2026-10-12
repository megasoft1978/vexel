# WO-UI-01 verification round 1 — FAIL

Candidate `58a1352`, base `e31084f`. Fresh read-only verifier, no repo writes.

Verdict: **not certified.** Machine gates AC-1/2/3/5/8/9 genuinely pass (81/81 CTests,
`./dev verify`, `./dev t --all`, both Python modules, ledger + wo_validate, `git diff --check`),
and the semantic-tree probe confirms the four tab nodes, the removed hint row and the three
status regions. But AC-4, AC-6, AC-7 and AC-10 are unsatisfied, and six required-implementation
items are partially unmet. Fix list for round 2:

1. Dead code: `viewer_chrome_editor.c` builds `"%s · Control-Tab"` but passes `spec->label`
   (no cycle hint exposed); `workspace_spec` is unused.
2. `inspector_group` is never applied by `viewer_editor_set_workspace`.
3. `docs/ui/README.md` (settings key) and `docs/ui/editor-interaction.md` (shortcuts, status
   regions) were not updated.
4. No cancel affordance in the status report region.
5. Readout omits zoom and document size.
6. The report text is duplicated into the readout ("Indexing 42%" appears twice).
7. Focused tests do not build the semantic tree, do not test the status regions or the removed
   hint row, and have no settings save/load round-trip; the pane check is trivially satisfied
   and the timeline branch never runs because `timeline_available` is 0.
8. `viewer_editor_set_workspace(NULL, ...)` segfaults while siblings handle NULL.
9. Ledger entries record `base_sha=head_sha=cddb869` instead of the base, and never record the
   candidate; `LEDGER.md` was not updated.
10. AC-4 pre-change failure and AC-6 keyboard workspace switch in `editor-smoke` are unrecorded;
    AC-7 requires a real save, not only in-memory materialisation.
11. `golden-changes.md` omits the four changed `manifest.json` files and claims all other
    fixtures are byte-identical, which is false for them.
