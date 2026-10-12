# WO-UI-02 AC-4: does the per-tab test actually constrain the tabs?

Mutation: in `src/app/viewer_chrome_inspector.c`, change

    if (tab == EDITOR_INSPECTOR_TAB_OBJECT) return section != 1U;

to

    if (tab == EDITOR_INSPECTOR_TAB_OBJECT) return true;

so the Object tab accepts the Fill/Stroke attributes it must not own.

Observed: `./build/mac-debug/bin/viewer_chrome_test` exits **46**, which is the
dedicated failure "the Object tab must not show Fill or Stroke". Reverting the
mutation restores exit 0. The assertion is therefore load-bearing rather than
vacuous.
