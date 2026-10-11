# Fresh verifier round 2 findings and fixes

The exact-commit read-only review of `fe545e0db5f75ca49e8127fd5e55f74aab3efd7e` found:

- Curve flatness used distance to the infinite chord line, so collinear controls outside the endpoint segment could hide curve overshoot. The flattening tolerance now uses distance to the finite segment, and the C regression checks a cubic with far-out controls.
- Shared numeric whitespace handling consumed commas in all SVG values. Commas are now consumed only by path data or the `viewBox` number-list parser, which rejects trailing/repeated separators. Focused negative cases cover root dimensions, rect dimensions, and repeated `viewBox` commas.
- The `docs/PERF.md` size entry disagreed with the recorded Release archive size. It now matches the freshly measured 24,144-byte shape library; CLI size is 53,392 bytes.

All frozen local acceptance commands pass on the corrected source. Fresh verifier round 3 is pending.
