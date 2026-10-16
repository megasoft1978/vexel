# ADR-0021: Native project recovery capsules

- Status: accepted for incremental implementation
- Date: 2026-10-13
- Decision owner: project owner
- Related: ADR-0018, ADR-0019

## Decision

Crash recovery for native projects stores a complete, independently validated
snapshot in a recovery capsule. A capsule records the originating document path,
the saved file's size and modification timestamp at the time edits began, the
capture time, and the complete encoded `.vexel` snapshot. It never replaces or
modifies the source project.

The project layer provides a bounded capsule codec and atomic file publication.
The application integration stores capsules in a private per-user Application
Support recovery directory, keyed by a digest of the standardized source path.
A capsule also contains the full path and must match it when opened; a key
collision or path mismatch is rejected. A later application work order wires
periodic background snapshots, retention, and recovery discovery to this API.

On restart, recovery is an explicit user choice. Vexel validates the capsule and
compares its saved baseline with the current source file. A matching baseline
can be restored as unsaved work. A changed or missing source is reported as a
recovery conflict; Vexel must not silently overwrite it. A successful normal
save or explicit discard removes the capsule. Atomic publication ensures a
crash during snapshot writing leaves either the previous complete capsule or
the newly published complete capsule.

CRC checks detect accidental corruption; they do not authenticate capsules.
Recovery remains local to the current macOS user account. It is not a backup or
synchronization service. Existing `.vexel` v1 content remains unchanged; the
capsule is a separate envelope and unknown project versions remain rejected.

## Consequences

Recovery snapshots contain the full project, including scene, native timeline,
metadata, and assets. The application must bound snapshot frequency and storage,
write off the UI thread, and make failures visible without blocking normal edits.
The per-user directory must be created with owner-only access. In-progress
application wiring and user interaction are not implied by the codec alone.
