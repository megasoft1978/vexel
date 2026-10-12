# ADR-0017: UI direction — Blender-aligned editor chrome

- Status: accepted
- Date: 2026-10-12
- Decision owner: project owner

## Context

The owner asked for the application UI to be reviewed against the current
interface of Blender and reworked toward a similar, more user-friendly design.

`docs/ui/blender-alignment-review.md` records the review: what the existing
chrome does well, the defects found in the committed UI fixtures and layout
code, the Blender conventions worth adopting, and the conventions we will
deliberately not copy. The review is based on the real chrome render, the
component gallery, the command palette, the timeline, the AppKit settings window,
the layout constants, and Blender's official manual and Human Interface
Guidelines for 4.2 LTS through 5.3.

## Decision

1. **Blender is the reference for chrome and structure**, not for editing
   semantics. Navigation, selection, transform and path editing stay familiar to
   Inkscape and Illustrator users; the *frame* around the canvas follows Blender.
2. **Task workspaces become the primary organising idea.** A topbar strip with
   `Draw`, `Nodes`, `Animate` and `Inspect` switches pane visibility and the
   active inspector tab. One document, one window, no mode switch on the editing
   model.
3. **One status bar.** The duplicated tool-hint row is removed. The status bar
   carries hints (left), progress and transient reports with a cancel affordance
   (centre), and readouts (right).
4. **The command palette is the universal entry point.** It gains menu paths as
   secondary text and a user-curated favourites group, and its result rows get a
   clear hierarchy distinct from any group header.
5. **Property surfaces become tabbed**, following Blender's vertical icon tabs in
   the Properties editor: `Object`, `Fill`, `Stroke`, `Document`, `Export`.
6. **Operations become tune-after-apply.** An Adjust-Last-Operation panel is added
   for primitives, path operations, transforms and export.
7. **Density becomes adjustable** through a compact/comfortable token set
   independent of the accessibility UI scale, defaulting to compact.
8. **Approximately ten semantic theme tokens stay.** Dark and light follow the
   macOS appearance. We do not adopt a per-editor theme matrix.

## Consequences

- The chrome gains structure (workspaces, pane visibility, tabs, footer, log) that
  must be covered by deterministic goldens in dark and light at 1× and 2×, plus
  semantic-tree assertions, exactly as the existing UI fixtures are.
- Workspaces store layout state, which means the settings model grows and needs a
  migration path for existing preference files.
- The custom-drawn editor chrome and the AppKit settings/welcome windows must be
  brought closer together in type scale, control metrics and colour so the
  application reads as one product; the review records this as defect E1.
- No editing semantic, document byte, renderer, or frozen oracle/golden changes as
  a result of this direction.
- The programme is executed as a sequence of frozen work orders
  (`work/WO-UI-*.md`), each independently verified; this ADR fixes the direction,
  not the schedule.

## Rejected alternatives

- **Copy Blender wholesale**, including its keymap editor, pie menus, five-flag
  visibility model and six editors per task. Rejected: the audience comes from
  Illustrator and Inkscape, and Blender's own guidelines document the
  discoverability cost of those mechanisms.
- **Redesign from scratch** without a reference. Rejected: the owner asked for a
  Blender-similar UI, and an established reference produces decisions faster and
  more defensibly than taste.
- **Leave the chrome as is** and fix only density. Rejected: the review shows the
  missing structure (workspaces, tabs, footer, tune-after-apply, status
  consolidation) is the larger usability gap, not the spacing.
