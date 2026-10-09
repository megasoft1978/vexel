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
