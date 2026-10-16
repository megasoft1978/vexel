**REJECT**

1. **Legacy pan icon geometry differs from the frozen C source.** Rust’s pan segments in [icons.rs](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust3-ui-draw/rust/crates/vexel-ui/src/icons.rs:70) use endpoints such as `(9,18)`, `(6,9)`, and `(18,9)`. The C geometry uses `(9,17)`, `(7,9)`, and `(17,9)` in [ui_icons_legacy.c](/Users/megasoft78/Desktop/Freelance/vexel/.worktrees/rust3-ui-draw/src/ui/ui_icons_legacy.c:15). This changes rendered pixels at every backing scale. Restore the C endpoints and add exact pixel assertions for the pan mask at scales 1–4; the current nonzero-pixel-count checks would not catch this mismatch.

The 53 Rust icon name/path records match the C definitions in a scoped source comparison. I did not run tests or builds, and make no runtime or full-product claim.

**Nonblocking gap:** The Rust hash checks assert nonzero values and scale/theme relationships, not fixed hash outputs. The scoped C assertions likewise check those relationships rather than fixed constants.