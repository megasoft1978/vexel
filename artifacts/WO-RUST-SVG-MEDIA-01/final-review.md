**REJECT**

- **P2 — Word spacing is lost between ASCII glyphs.** [text_geometry.rs:88](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text_geometry.rs:88) advances across all consecutive ASCII bytes during the first glyph. For `A B`, the space never increases `word_offset`, although the cursor includes it. Advance exactly one UTF-8 code point per glyph; test the final glyph’s coordinates.

- **P2 — Initial relative positioning is applied twice.** [text_geometry.rs:368](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text_geometry.rs:368) includes `dx[0]/dy[0]` in the start position, then geometry accumulates them again when the corresponding absolute list is absent. Test `<text dx="2" dy="3">A</text>` and apply each offset once.

- **P2 — CDATA incorrectly undergoes entity decoding.** [text_geometry.rs:330](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text_geometry.rs:330) sends both text and CDATA through `normalize_text`. Consequently, `<![CDATA[A&amp;B]]>` changes content, and a literal unmatched `&` can fail. Preserve CDATA literals while applying the required whitespace handling.

- **P1 — Whitespace normalization can allocate infallibly.** [text.rs:66](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text.rs:66) reserves only `decoded.len()`, but an inherited pending space can produce `decoded.len()+1` bytes. The subsequent `push` can invoke an aborting allocation. Reserve the checked maximum fallibly.

- **P2 — Variable parsing loops remain uncancellable.** [text.rs:99](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text.rs:99) scans position attributes without polling or charging work; family parsing and normalization have the same gap. Add controlled production variants and cancellation tests inside long inputs. Interrupting every existing callback boundary does not cover these loops.

- **P2 — Outline diagnostics lose the authored offset.** [text_geometry.rs:468](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text_geometry.rs:468) supplies offset `0`; cubic flattening and control-error conversion also return zero. Thread the current run offset through geometry generation and assert offsets on late geometry failures.

- **P2 — Admission consumption is not tied to the returned shape.** [publication.rs:163](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/publication.rs:163) checks only suffix identity, mapping, counts and image index. A delegate can cancel the supplied admission and use ordinary append; equal-sized replacement geometry also passes. Require a verifiable admission-to-shape association and test both bypasses; report a shared-contract gap if necessary.

- **P2 — Successful requests can commit an infinite cursor.** [text_geometry.rs:439](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/src/text_geometry.rs:439) scales and adds finite provider values without checking the result. A finite near-maximum advance overflows, including for hidden/no-outline text. Validate the computed cursor before assignment and exercise rollback.

The logs record successful check and **58 passing tests**; I did not rerun state-writing commands. All 18 source/test line counts match the audit and remain below 800. Historical predicate retention and protected-path provenance cannot be independently established from the permitted current files alone.

Downstream limitations remain: native providers, the real style/composition adapter, mixed-renderer fidelity and application/UI validation are outstanding ([README.md:74](/Users/megasoft78/Desktop/Freelance/vexel/rust/crates/vexel-svg-media/README.md:74)).