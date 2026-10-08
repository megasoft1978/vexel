# 01 — Product scope

## Vision and wedge

“Open, inspect and lightly edit huge SVGs without waiting for the entire document or losing source data.” Initial users: GIS/cartographers, scientific visualization authors, and selected laser/plotter workflows. Position as a large-SVG workbench, not a parity replacement. Animation is later, still SVG.

## MVP IN

- Open SVG (plain UTF-8 XML initially); bounded file open with progress/cancel.
- Progressive viewport render; pan, zoom, fit, selection and selected-object bounds.
- Select/move/scale/rotate; basic path node/handle edit.
- Fill, stroke, opacity, solid/linear/radial gradient basics.
- Layers/object tree with visibility/lock/order; virtualized XML read-only inspector.
- Undo/redo transactions, save-as, recovery journal, source-preserving save.
- Mac native file dialogs, clipboard, menus, IME, keyboard, text shaping, accessibility for core UI.
- Unknown element and Inkscape/Sodipodi metadata preservation; unsupported feature diagnostics.
- Safe SVG limits, no network loading by default, no script execution.

## MVP OUT

- Full Inkscape tool/LPE/extension parity; booleans; mesh gradients; advanced filters/masks/markers parity; scripting or external resources; arbitrary CSS editor; print/PDF/CMYK workflows; collaboration; browser-perfect typography; full tablet pressure; plugin ecosystem; animation timeline; Lottie export.

## Non-goals

- Promise arbitrary 1 GB files are fully editable in RAM.
- Modify or fork Inkscape code under a clean-room premise.
- Drop unsupported XML or silently flatten all CSS/styles.
- Add a widget framework, embedded browser, or second UI implementation.
- Optimize synthetic files while ignoring real user files.

## Success metrics (proposed gates)

M0 targets: 10 MB simple SVG first visible pixel <1 s; 100 MB map-like SVG <2 s; parser does not block event loop >100 ms; cached pan/zoom p95 <33 ms; peak RSS measured and reported; unchanged save preserves all original bytes where no edit requires parent rewrite. M10: five user pilots; 100k simple objects selection/move p95 <50 ms; no data-loss corpus cases; ≥90% selected supported static fixture images within declared tolerances; 0 silent unsupported drops. App size initially measured, not promised; stretch ≤25 MB macOS app package single architecture and ≤10 MB stripped core binary. All gates must be reported by exact hardware, OS/build/corpus hashes.

## Kill criteria

- Current Inkscape or Donner within 20% on primary user workflows with comparable fidelity and package constraints, unless users identify another material advantage.
- Under 5 of 15 interviewees will provide a file/pilot or rate wait/round-trip problem as top-three pain.
- Source-preserving editing architecture requires eager full-document normalization or loses unknown metadata.
- 100 MB visible-first-paint and no-stall gates remain out of reach after two renderer/parser iterations.
- Accessibility/IME design cannot be supported with the available team.
- No committed maintenance/funding model for cross-platform release after MVP.
