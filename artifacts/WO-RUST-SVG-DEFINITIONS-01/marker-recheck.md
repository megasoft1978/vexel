**ACCEPT — marker repair.** The previous full review’s sole marker blocker is resolved:

- Nonpositive `markerWidth` and `markerHeight` return `Limit(Shapes)` at the marker name offset, matching the original C behavior.
- Malformed values return `Number` at the attribute value offset. Overlimit values retain the limit error; positive explicit values and defaults remain guarded.
- The four regressions call the production `parse_marker_attributes` API with tokenizer output and assert both error kind and source offset. They use no fake wrappers.
- The repair report records 36 prior assertions retained, four regressions added, 40 checks passed, offline verification, and work-order validation.

I did not find the named `marker-review-diff.json`, `full-review.md`, or `previousfullreview` files in this worker tree, so I could not independently confirm the claimed exact protected-path audit or changed-file set from those artifacts. This scoped acceptance does not complete the whole source gate; root fresh-main review and checks remain required.