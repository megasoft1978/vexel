# Lossless no-op source save

`source_save_noop` accepts borrowed source and destination byte slices plus
caller-owned token storage. It applies the XML tokenizer's 16 MiB and 256-depth
defaults before it invokes filesystem code. A successful parse writes the exact
source slice; there is no token serializer. Tokenizer errors never inspect or
mutate the destination.

The macOS adapter accepts non-empty, NUL-free paths no longer than 1023 bytes,
requires an existing regular destination, and creates an exclusive temporary
file in its parent directory. It handles interrupted and partial writes,
applies the original POSIX mode bits, fsyncs and closes the temporary file,
then atomically renames it over the destination. Rename is the commit point.
Failures before rename leave the old target untouched and remove the temporary
file. Directory fsync happens after commit; failure then returns
`SOURCE_SAVE_DURABILITY` with `renamed=true`, and does not imply the old target
is still present. File metadata beyond POSIX mode bits is out of scope.

The optional hooks replace only write and rename operations for deterministic
tests. Null hooks use native macOS calls. The caller keeps all slices and token
storage alive for the duration of the call and owns their storage throughout.
