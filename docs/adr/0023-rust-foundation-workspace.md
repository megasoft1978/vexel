# ADR0023: Rust foundation workspace

Accepted2026-10-14 under the full-port owner direction. Rust1.99.0/2024/resolver3,
MPL2, std-only/internal dependencies, macOS13 AppleSilicon only. Base/geometry/
xml/json/project/raster and developer CLI members are scaffolding for ports.
Domain crates forbid unsafe; native FFI belongs to a later audited platform crate.
No external dependency adoption. Shared Span(start,end) uses checked arithmetic
and slices. Point/Affine are f64 SVG geometry, compose maps right first.
Coordinator owns manifests/lock/contracts. Build success is not product parity.

Exact compiler pin1.99.0 is recorded. The host stable alias resolves to it, so
checks use +stable after exact-version verification without global installs.
Build/cache/tmp output repo-local. Six disjoint foundation owners port modules
behind these interfaces, with focused checks and original fixture preservation.
