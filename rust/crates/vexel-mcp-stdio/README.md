# vexel-mcp-stdio

A blocking, caller-owned line transport for the accepted `vexel-protocol`
session. It reads LF or CRLF frames incrementally, accepts one final frame at
EOF, and caps each frame at `vexel_json::MAX_BYTES`. It never reads from global
stdin/stdout or starts a process. Empty and oversized frames fail before
protocol dispatch; oversized frames stop the stream without draining it.

`serve` dispatches through `vexel_protocol::handle`; `serve_async` dispatches
through `handle_async`. The latter does not spawn tasks or supply an executor.
The caller supplies and owns the streams, protocol session, and both backends,
and is responsible for composing those with an application lifecycle. A
nonempty protocol response is written byte-for-byte, followed by LF and flush;
notifications that produce no response remain silent. I/O and protocol errors
are returned to the caller, with no diagnostics written to the protocol stream.
