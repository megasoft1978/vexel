# Base byte slices and arena

`base.h` exposes checked `size_t` arithmetic, immutable and mutable byte slices,
and a bounded chunked arena. The module uses C17 and has no dependencies beyond
the C library.

## Arithmetic and slices

`base_size_add` and `base_size_mul` return `true` and write the result when the
operation fits in `size_t`. A null output pointer or overflow returns `false`;
when an output pointer is valid, it is set to zero on failure.

`base_bytes` and `base_mut_bytes` are pointer-plus-length views. A null pointer
is valid only for a zero-length slice. Slice operations do not inspect
terminators or infer a length. Sub-slicing checks `offset <= len` and
`length <= len - offset`; failure clears a valid output to `{NULL, 0}`. A
successful zero-length sub-slice is also represented canonically as
`{NULL, 0}`. Equality returns false for invalid slices or unequal lengths,
true for any two valid empty slices, and otherwise compares exactly `len`
bytes. The caller owns the memory and must keep it valid for the duration of
each operation.

## Arena ownership and lifetime

Create an arena with a nonzero `initial_chunk_bytes`, a `capacity_limit`, and
either both allocator callbacks or neither. With no callbacks, the arena uses
`malloc` and `free`. A custom allocator must return storage suitably aligned
for any C object, and its context must remain alive until the arena is
destroyed. Every successful allocation, including arena metadata and chunks,
is released through the matching free callback exactly once.

Chunk allocation is lazy. Each chunk has a payload capacity of at least the
initial chunk size (or the request size, when larger), subject to the remaining
capacity limit. The limit counts the sum of chunk payload capacities and
excludes chunk headers and alignment slack. Chunk payload starts on a 4096-byte
boundary. `base_arena_get_stats` reports retained chunk count, total payload
capacity, and bytes consumed including alignment padding.

Push alignment must be a power of two from 1 through 4096. Invalid arena or
alignment returns `BASE_STATUS_INVALID`. A zero-size push succeeds with a null
output pointer and consumes nothing after its alignment has been validated.
Nonempty pushes return `BASE_STATUS_LIMIT` when another chunk cannot fit within
the total payload limit, `BASE_STATUS_OVERFLOW` if allocation-size arithmetic
cannot be represented, and `BASE_STATUS_OOM` if the allocator fails. On any
failure the output pointer is null and arena chunk/capacity/usage state is
unchanged. A successful pointer stays valid until reset or destroy.

Reset sets every retained chunk's used count to zero and keeps all chunks for
reuse. Destroy frees all chunks and arena metadata. The arena is single-owner,
not thread-safe, and must not be used after destroy. Callers must not use pushed
pointers after reset or destroy.

## Error and thread behavior

All operations that can fail report a boolean or `base_status`; they do not
abort or assert on caller input. Null destroy and reset are no-ops. Statistics
with a null output return false; when the arena is null, a valid output is
cleared and the call returns false. No operation allocates memory except arena
creation and a push that needs a new chunk.
