# ADR-0018: Vexel product goal and completion gates

- Status: accepted
- Date: 2026-10-13
- Decision owner: project owner
- Supersedes conflicting product-scope and sequencing statements in ADR-0016 and the round-4 roadmap; preserves platform and implementation locks in `OWNER-DIRECTIVES.md` and accepted architecture ADRs.

## Decision

Vexel is a high-quality, macOS-first vector drawing and editing application with
first-class 2D cartoon animation authoring. It aims to be a serious alternative
to Inkscape for creating and editing vector artwork, while adding a coherent
animation workflow and live integration for Claude Code, Codex, and compatible
MCP agents. The product target is ambitious; parity is not claimed until the
documented coverage and quality gates below are met.

1. **Native macOS product.** Initial platform remains Apple Silicon macOS 13+.
   C17, custom-drawn editor UI, owned SVG parser/rasterizer, minimal pinned
   dependencies, MPL-2.0, and platform seams remain governing constraints.
2. **Modern, purpose-built interface.** Use task-focused workspaces, flexible
   editor areas, and contextual properties as design references from Blender
   and Figma, and timeline/onion-skin patterns from established animation tools
   such as Krita and Blender Grease Pencil. Follow macOS conventions for command
   placement, keyboard use, accessibility, and discoverability. Build an
   original Vexel interface; do not copy another product wholesale. Existing
   Blender-aligned direction in ADR-0017 remains valid within this synthesis.
3. **SVG interoperability.** SVG and Inkscape-authored SVG are the import and
   interchange formats. Preserve Inkscape namespace metadata, unsupported
   constructs, and unknown source data whenever an edit can retain them. Report
   unsupported or approximated behavior specifically; never silently discard
   content. Export behavior and unsupported-feature limits must be documented
   and measured against full documents.
4. **Native project.** Introduce a versioned `.vexel` project format for editable
   vector scene state, animation, assets, and project metadata. SVG remains the
   interchange surface. Package/container structure, schema, migration,
   portability, and compatibility policy require a separate design ADR before
   implementation.
5. **Animation.** The intended authoring workflow includes property/timed
   keyframes, frame-by-frame vector drawings and exposures, onion-skin review,
   deterministic playback, and animation export. Existing SMIL/CSS playback and
   timeline behavior are starting capability, not proof that these authoring
   workflows are complete. SVG SMIL/CSS round-trip and native animation support
   are tracked separately where semantics differ.
6. **Large-document responsiveness.** Measure full-file first visible content,
   interaction latency, memory/RSS, background work, and cancellation separately
   from complete-frame rendering fidelity. Rebaseline the prior proposed targets
   on named Apple Silicon hardware: first visible content under 2 seconds for a
   representative 100 MB document, main-thread pauses under 100 ms, and pan/zoom
   p95 below 33 ms. These are targets, not achieved results. Every benchmark
   includes file hash, hardware, build, warm/cold protocol, repeated samples, and
   raw results; bounded resource failures must be explicit and recoverable.
7. **Live MCP integration.** Keep local stdio and explicit file grants as the
   default. MCP should let an authorized agent inspect and edit the same live
   document as the user, including scene, selection, animation, save/export,
   and render operations. Agent actions and results must be visible in the app,
   with clear progress, errors, bounded cancellation, undoable transactions, and
   user-controlled file access. Maintain documented protocol interoperability.
   Any remote transport requires a separate authenticated threat model and
   owner decision; do not expose an unauthenticated listener.
8. **Recorded product demonstration.** Product completion includes a polished,
   accessible recording of Codex using the live MCP integration to create a
   character and animate it in Vexel while the user can see the application
   update. Use this end-to-end example to demonstrate the product on its website
   and in the repository README. The recording is a final showcase deliverable;
   it does not replace automated verification or claim unfinished capabilities.

## Completion gates

Vexel is ready for a high-quality release only when evidence supports all of the
following:

- **Fidelity and preservation:** full-document corpus/reference renders report
  per-file outcomes; supported documents match declared visual tolerances;
  unsupported data survives round trips; no silent source/data loss.
- **Large files:** reproducible benchmarks meet the rebaselined responsiveness
  and memory targets on named hardware, with cancellation and limits verified.
- **Drawing and animation:** core create/edit/organize/style/transform/save and
  frame-by-frame/timeline/onion-skin/playback/export workflows are usable and
  undoable.
- **Projects and recovery:** versioned native projects reopen with artwork,
  animation, and assets intact; compatibility/migration and interrupted-save
  recovery behavior are demonstrated.
- **UI quality:** workspace and inspector workflows are coherent, keyboard
  complete, accessible, polished at supported display scales, and reviewed with
  representative users before release.
- **MCP:** Claude Code, Codex, and a protocol test client can inspect and drive a
  live session; changes appear in the application; permission boundaries,
  progress, cancellation, error recovery, and undo are demonstrated.
- **Final acceptance:** only after planned feature implementation, run the
  comprehensive automated UI/end-to-end and release verification pass, then
  review the live product across the gates above. Focused checks required by
  individual work orders continue during development.
- **Showcase evidence:** after the demonstrated drawing, animation, and MCP
  workflows pass final acceptance, record and review the live Codex/Vexel
  character-creation and animation example, then publish/link it from the
  product website and README.

## Basis and consequences

The owner's expanded goal and the repository inventory/research are captured in
[`docs/product/vexel-goal-and-delivery-plan.md`](../product/vexel-goal-and-delivery-plan.md).
Blender documents task-specific [workspaces](https://docs.blender.org/manual/en/latest/interface/window_system/workspaces.html)
and animation [keyframe channels](https://docs.blender.org/manual/en/4.4/editors/dope_sheet/introduction.html);
Figma documents contextual [layer properties](https://help.figma.com/hc/en-us/articles/360039832014-Design-prototype-and-explore-layer-properties-in-the-right-sidebar).
Apple's [toolbar guidance](https://developer.apple.com/design/human-interface-guidelines/toolbars)
and Krita's [animation timeline](https://docs.krita.org/en/reference_manual/dockers/animation_timeline.html)
inform platform-aware composition. Blender documents [Grease Pencil frame
animation and onion skinning](https://docs.blender.org/manual/en/3.4/grease_pencil/animation/introduction.html).
Inkscape describes its SVG basis and editor metadata in its [FAQ](https://inkscape.org/es/learn/faq/)
and [SVG overview](https://inkscape.org/en/develop/about-svg/); the [W3C SVG
specification](https://www.w3.org/TR/SVG/) defines interchange semantics. The
[MCP 2026-07-28 update](https://blog.modelcontextprotocol.io/posts/2026-07-28/)
is the protocol reference already recorded by `docs/mcp.md`. Apple's
[document-package model](https://developer.apple.com/library/archive/documentation/CoreFoundation/Conceptual/CFBundles/DocumentPackages/DocumentPackages.html)
is a reference for considering a multi-file native project, not a selected
container design. Apple recommends profiling graphics workloads with tools
described in [Metal performance analysis](https://developer.apple.com/documentation/xcode/analyzing-the-performance-of-your-metal-app/);
GPU work is not assumed necessary before measurement.

These decisions make SVG fidelity, native animation/project integrity,
large-document responsiveness, and visible agent control first-class product
requirements. Existing renderer, UI, animation, and MCP work remains valuable;
the delivery plan identifies specific gaps and dependencies instead of
restarting completed work. The comprehensive UI/end-to-end acceptance pass is
the final gate, after feature implementation, per the owner's request.
