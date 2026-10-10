# UI lane

The editor chrome demo is composed from a retained widget tree and uses
logical pixels for layout. Interactive roles own bounded UTF-8 labels/values;
focus traversal wraps through enabled buttons, checkboxes, text fields, and
sliders. Command bindings are unique key/modifier pairs. Semantic snapshots
copy stable node and parent IDs, role, labels, values, and state flags. The
current golden fixture checks deterministic 1280x800 screenshots for dark and
light themes. Regenerate only after visual review with
`python3 tests/ui/verify_chrome.py --update`.

M6-02 adds widget constructors for buttons, checkboxes, text fields, sliders, and
rows. Tab/Shift-Tab move focus through enabled interactive controls; Enter/Space
activate commands, and Space toggles checkboxes. Stable semantic snapshots copy
node IDs, parent IDs, roles, labels, values, and disabled/checked/focused states.

The editor composes the toolbar, scrollable structure outline, inspector
attribute rows, text editor, and status bar from the same retained tree. Its
semantic snapshot is stable for a given document and viewport. The editor's
scalar paint path writes into the viewer's existing frame buffer and uses
bounded caller storage; dark and light 1280x800 editor chrome goldens live in
`tests/editor/chrome-golden/`.
