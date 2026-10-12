# WO-GAP-UI-02: does inspector-tab-reachability bite?

Test: `tests/editor/inspector_tab_reachability_test.c` (CTest `inspector-tab-reachability`).
It exits 10 when any attribute is not on exactly one tab. Both mutations were
applied to `src/app/viewer_chrome_inspector.c` and reverted afterwards
(`git diff` shows only the real fix below).

## Mutation 1: Fill also lists stroke attributes
Added after the Document line of `inspector_tab_accepts`:

    if (tab == EDITOR_INSPECTOR_TAB_FILL) return section == 1U;

Result, exit 10:

    attribute 'stroke' is on 2 tabs: Fill Stroke
    attribute 'stroke-width' is on 2 tabs: Fill Stroke
    attribute 'stroke-opacity' is on 2 tabs: Fill Stroke
    attribute 'stroke-linecap' is on 2 tabs: Fill Stroke
    attribute 'stroke-linejoin' is on 2 tabs: Fill Stroke

## Mutation 2: Object skips an attribute it owns
Changed the Object line to
`return section != 1U && !(name is "data-note");`.

Result, exit 10:

    attribute 'data-note' is on 0 tabs:

## Real gap found and fixed (not a mutation)
Before the fix (`before-fix.txt`, exit 10): `attribute 'id' is on 3 tabs:
Object Fill Stroke`. The Name (id) field, text content and add/remove fields
were built on every filtered tab. Fix: `editor_build_inspector_panel` returns
after the attribute rows when the active tab is not Object. The assertion is
unchanged. After the fix and with both mutations reverted the test exits 0.
