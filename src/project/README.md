# Native project container codec

## Native recovery capsule v1

`project_recovery.h` wraps a complete encoded project bundle in an independent
recovery record. Its fixed 72-byte little-endian header stores the capsule
version and header size, source path length, baseline source size and mtime,
capture time, nested bundle length, and CRC-32 checks for the path, payload, and
combined record. The UTF-8 path is followed by the complete `.vexel` v1 bundle.
The capsule cap is derived from the bundle, header, and 1023-byte path limits.
CRC detects accidental corruption only; it is not authentication.

Decode returns borrowed views into the capsule bytes, which must outlive every
view. An optional expected source path must match byte for byte. The storage API
atomically creates or replaces only a caller-selected regular recovery target;
it refuses the source path and never changes the source project's saved state.
The app resolves a private macOS Application Support target and schedules a
best-effort background capture every 30 seconds while a dirty native project
has a stable saved-file baseline. Scene bytes are materialized from an
immutable piece-table snapshot on the worker; timeline bytes are pinned across
history eviction; project asset descriptors are copied while immutable asset
payloads remain borrowed. Before and after publication, the app checks the
source file stamp and drops a capsule if that baseline changes. Capture does
not mark edits saved or alter undo history. The app discovers capsules when
opening an existing native source, validates the exact embedded path and full
bundle, offers restore/discard/cancel choices, reports source conflicts, and
removes capsules after discard or a successful save. Missing-source discovery
is unsupported because store lookup requires an existing canonical source.
Retention/expiry, project-version migration policy, measured peak recovery
memory on very large projects, and broad UI/E2E acceptance remain open.

`project_bundle.h` provides the bounded format-v1 codec. The file starts with
56 bytes: magic `VEXEL\r\n\x1a`, version u32, header-size u32, metadata length
u32, asset count u32, SVG scene length u64, timeline length u64, CRC-32/IEEE
u32 for metadata/scene/timeline, and zero flags u32. All integers are little
endian. Payload order is metadata JSON, exact SVG source, timeline JSON, then
asset records. Each record contains name length u16, media type length u16,
CRC-32/IEEE u32 over the name, media type, and data, data length u64, then the
name, media type, and data bytes.

The codec is allocation-free: decode returns borrowed byte slices into the
encoded input; encode writes into caller storage. The input must outlive and
remain unchanged while a decoded view is used. Encoder output must not overlap
any input slice. The codec does not normalize JSON/XML, materialize SVG animation,
extract assets, or access the filesystem. Callers must validate payload syntax
before opening a project and must use a separate atomic-save path for writes.
Unknown versions are rejected. Any incompatible layout or meaning change needs
a version bump and explicit migration code.

Bounds are declared in `project_bundle.h`: project 1 GiB, metadata 1 MiB, scene
256 MiB, timeline 64 MiB, 1024 assets, each asset 256 MiB, and total asset data
512 MiB. CRC-32 detects accidental corruption only; it does not authenticate a
file. Names are single safe identifiers, not paths.

## Native animation timeline v1

`project_timeline.h` validates the optional timeline JSON independently of the
MCP JSON parser, whose lower input/token limits are intended for protocol
messages. Decode borrows the payload and returns views into its original bytes;
the input must remain alive and unchanged. Empty payload means an empty native
timeline. Unsupported non-empty schema versions are reported so existing
container callers can preserve them as opaque bytes.

Version 1 uses integer ticks, ordered drawing exposures, and typed property
keys. Targets are unique SVG `id` strings, never process-local document handles.
Drawing cels reference named `image/svg+xml` project assets. The reader checks
payload bounds, timing arithmetic, ordering, duplicate IDs/keys, known fields,
and caller-provided sorted scene-ID and validated asset inventories. Track and
item accessors expose byte-exact source spans; decode JSON string contents with
`project_timeline_string_copy` before interpreting escaped values. Timeline
loop playback and property-track evaluation remain later animation work. SVG
SMIL animation remains separate source-based interchange data.

`project_timeline_validate_bundle` classifies the payload before allocating a
scene-ID inventory. Empty data is an empty timeline; a recognized v1 timeline is
validated against unprefixed SVG IDs (entity-decoded, unique, at most 256 bytes
each and 1,000,000 total) and project assets; valid unsupported schemas/versions
remain opaque and byte-exact. Malformed JSON is rejected. The caller's
cancellation callback is polled during long JSON scans, XML visits, and timeline
validation. The returned timeline view borrows the bundle's input bytes, so the
bundle owner must outlive every use of that view.

During native-v1 project load/save, every distinct SVG asset referenced by a
drawing cel is decoded with the owned shape parser. Duplicate/unresolved local
references and unsupported content such as scripts or external resources fail
with the asset name and source offset; no resource is fetched and source bytes
are never rewritten. SVG SMIL remains source-based interchange content. Assets
not referenced by drawing cels (including imported provenance) remain opaque
and are preserved. The decoder's existing SVG source, token, shape, and edge
limits apply to each referenced asset. The streaming reference preflight caps
the local ID inventory at 65,536 IDs of at most 256 decoded bytes each and
checks cancellation between visitor chunks and inside shape traversal. Load jobs pass their task token; project save exposes a thread-safe cancellation hook. Href and CSS reference-bearing
attribute/text values are capped at 4 KiB during preflight.

### Native timeline authoring model

`project_timeline_edit.h` owns a copy of a recognized, valid v1 timeline and
offers drawing-track creation, sorted cel upsert/removal, property-track
creation, sorted property-key upsert, and undo/redo. It borrows the scene-ID and
asset validation context only for each call. A new drawing track starts with
the required tick-zero exposure and must refer to an existing SVG target and
SVG asset. A property track starts with a tick-zero value and its property,
value type, and interpolation must match ADR-0020. JSON strings generated by
edits are escaped and UTF-8 checked; raw key values are bounded and the timeline
decoder validates each candidate before it becomes current. Edits replace only
the affected array spans, preserving other root fields and untouched
track/item source bytes.

History defaults to 16 edits and 128 MiB of owned timeline snapshot payloads,
with hard limits of 1024 edits and 512 MiB. The byte count includes current,
undo, and redo snapshots; it excludes bookkeeping and temporary candidate
buffers. Oldest undo snapshots are evicted to stay within the configured caps.
An operation that cannot fit, including the current and candidate payloads, is
rejected without changing the current timeline, undo cursor, or redo branch.
Source views borrow the edit model and are invalidated by a successful edit,
undo, redo, or destroy. Live MCP editing uses the project session. Save As from
an edited SVG creates a valid native-v1 project and installs its editable
zero-track timeline only after the encoded project is published successfully.
The empty timeline exposes opacity and transform key actions for a selected SVG
element with a stable ID.

`project_timeline_session.h` attaches that edit model to a loaded native
project. Validation can retain its already-built scene-ID index and asset-name
inventory as an owned context, avoiding a second SVG ID traversal on load. The
context's asset bytes still borrow from the loaded bundle. Sessions can refresh
the context against a changed scene/bundle; a failed refresh leaves the prior
context usable. The app owns a session only for validated native-v1 timelines,
and project save serializes its current timeline bytes after validating them
against the current scene and bundle assets. Opaque future and empty timelines
remain passthrough data with no edit session. Live MCP editing uses that same
session; native timeline UI remains subsequent work.

### Native frame composition

`project_animation_render.h` renders one integer tick from an already decoded
scene, decoded project bundle, and validated timeline view. Callers pass an
empty, zero-initialized output tile. Visible property tracks evaluate step,
linear, or smoothstep-cubic keys at that integer tick, holding endpoint values
outside their key range. Scalar, RGBA, transform2d, and visibility values are
applied to a temporary source-preserving scene snapshot; saved scene and
timeline bytes remain unchanged. Invisible tracks are skipped. The selected
drawing-cel exposure is the last cel whose start tick is at or before the
requested tick. Visible drawing tracks composite in timeline order using source-over into a
premultiplied RGBA tile. Asset SVG documents have independent ID scopes and are
rendered in their own root coordinate space. V1 has no target-relative drawing
placement metadata, so the renderer currently requires exact pixel and
viewBox-coordinate equality with the scene instead of stretching or cropping.
The scene, bundle, and timeline bytes remain borrowed and unchanged; the output
tile belongs to the caller. Total rendered RGBA bytes across the scene and
active layers are capped at 512 MiB per frame. Loop sampling, onion skin,
export, and app playback/UI are not part of this API.

`project_timeline_session_decode` returns a view borrowing the current session
bytes; edits, undo, redo, and session destruction invalidate it.
`project_timeline_session_render_frame` validates and decodes the current edited
timeline against the supplied scene and bundle on each call, then invokes the
integer-tick renderer without retaining a borrowed view across calls. The
scene and bundle must describe the same current project. Render output is
caller-owned on success and remains empty on failure. Session and project
bytes are read-only during rendering; drawing assets still use SVG root
coordinates because target-relative placement is not defined in v1.

The native timeline also exposes a first property authoring action: “Key
Opacity” records the selected SVG element's presentation opacity at the
playhead, using its stable SVG id and an undoable native property key. New
tracks use linear scalar opacity and seed tick zero; later-frame track creation
is one undo step. Inline-style-only opacity is reported as unsupported by this
action. Other property editing and drawing-cel authoring remain follow-up work.

Live MCP clients can add a complete SVG drawing asset to the active native
project with `native_timeline_add_drawing_asset`, then create a drawing track
and cels that reference its safe unique name. The app keeps original bundle
bytes intact and owns new assets in a bounded overlay until the next save;
overlay assets are included in bundle asset lookup, frame rendering, and
container serialization. Asset-only edits participate in unsaved-change
warnings and are retained after Save or Save As to `.vexel`.

The app's native playback view uses integer floor sampling from the playback
clock and clamps the end-of-duration display sample to the final valid tick.
The timeline can create opacity and transform keys for a selected SVG element
with a stable ID, add drawing tracks/cels, and retime native items by dragging.
Other property editing and richer key/cel operations remain follow-up work.
The standard Undo/Redo actions prefer native timeline history in Animate when
the requested direction is available, then fall back to SVG edit history.
Other workspaces use SVG history. The toolbar, keyboard shortcuts, menus, and
command availability share this routing. The SVG Undo History popover lists
SVG edits only and does not present one merged chronological history. The viewer
refreshes the session validation context after scene edits and uses its
refreshed borrowed timeline view for each frame, avoiding full project
revalidation on every playback sample. The renderer still evaluates drawing
assets in root SVG coordinates.
# Onion skin preview

Native animation onion skins are editor previews shown only while playback is
paused. They composite neighboring integer frames behind the selected frame in
the timeline preview. Project frame rendering, MCP render/export output, SMIL
animation, and saved SVG pixels remain unchanged; the project stores only the
before/after counts and opacity preference.

`project_timeline_source_lease` pins the immutable current timeline byte buffer
without copying. It survives edits, undo-history eviction, and edit-model
destruction; the application releases leases on the UI thread after worker
completion and before destroying the owning document session.
