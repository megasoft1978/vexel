# ADR-0019: Vexel native project container v1

- Status: accepted for the codec foundation
- Date: 2026-10-13
- Decision owner: project owner
- Related: ADR-0018

## Decision

Vexel's native project is a single versioned `.vexel` file. Version 1 is a
bounded little-endian container holding a JSON metadata payload, exact SVG scene
source, a JSON timeline payload, and named raw assets. Keeping SVG source as an
opaque exact payload preserves Inkscape metadata and unrecognized constructs;
the native timeline can evolve independently of SVG animation interchange.

The fixed 56-byte header contains magic bytes `56 45 58 45 4c 0d 0a 1a`, version,
header size, metadata length, asset count, scene and timeline lengths, three
CRC-32/IEEE checksums, and zero flags. It is followed by metadata, scene,
timeline, then asset records. Each record has name/media-type lengths, a CRC-32
checksum over all three payload fields, a data length, and the exact name, media
type, and data bytes. All integers are little-endian. The full field table and limits live in
`src/project/README.md`.

Initial limits bound metadata to 1 MiB, scene to 256 MiB, timeline to 64 MiB,
assets to 1024 entries and 512 MiB aggregate, each asset to 256 MiB, and the
entire encoded project to 1 GiB. Asset names are unique safe single path
components. Decoders reject unknown versions/flags, checksum mismatches,
truncation, trailing data, duplicate names, unsafe names, and over-limit sizes.
The codec borrows input bytes and writes to caller-provided buffers; it does not
allocate or perform file I/O.

## Compatibility and limitations

Version 1's payloads are opaque to the container. Callers validate metadata and
timeline JSON and the scene SVG before use. A change to header layout, field
meaning, or incompatible payload semantics requires a new format version and an
explicit migration path; never silently reinterpret an unknown version. Future
work must wire file open/save, editor dirty state, atomic replacement, asset
import, compatibility UI, and recovery before claiming native project support.

CRC-32 detects accidental corruption but is not cryptographic authentication.
This format does not fetch resources or execute content. Asset names are
identifiers and are never extracted as paths by the codec.

## Rationale

A self-contained binary container supports bounded sequential I/O and avoids a
new archive dependency or path traversal during extraction. Raw sections keep
the codec independent from SVG and animation parsers and preserve bytes exactly.
The limits accommodate the project's large-document target while keeping
explicit ceilings. Apple document-package conventions remain useful for future
UX, but the chosen initial format is one file for straightforward sharing,
atomic save, and backup.
