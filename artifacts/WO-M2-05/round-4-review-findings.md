# Fresh verifier round 4 findings and fixes

The exact-commit read-only review of `5a19d11d8e4342408b3496b1d6259f0a93c70f1b` found:

- Rectangles without `width` or `height` were treated as empty shapes. Both dimensions are now required; omitted x/y still default to zero. Focused diagnostics cover each missing dimension.
- Processing instructions were silently skipped, allowing an `xml-stylesheet` instruction to be ignored. Processing instructions and CDATA now produce unsupported-feature diagnostics, and non-whitespace text is rejected. A stylesheet PI case is in the focused diagnostics test.

All frozen local acceptance commands pass on the corrected candidate. Fresh verifier round 5 is pending.
