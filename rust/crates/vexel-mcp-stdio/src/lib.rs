#![forbid(unsafe_code)]
//! Bounded, blocking line framing around the accepted Vexel MCP protocol.
//!
//! The caller owns both streams, the protocol session, and the backends. Each
//! newline-delimited frame is dispatched once; LF and CRLF are accepted, and
//! one final unterminated frame is processed at EOF. Frames are limited to
//! `vexel_json::MAX_BYTES`. Empty/oversized frames and I/O/protocol failures
//! stop serving with an error. Responses retain the protocol's exact bytes,
//! followed by LF and a flush. Notifications with empty responses write nothing.
//! These functions block on caller-provided streams; callers compose lifecycle,
//! process startup, and backend policy around them.

use std::io::{self, BufRead, Write};
use vexel_json::MAX_BYTES;
use vexel_protocol::{AsyncBackend, ProtocolBackend, ProtocolError, ProtocolSession};

/// Stdio framing, protocol, or output failure.
#[derive(Debug)]
pub enum StdioError {
    Io(io::Error),
    EmptyFrame,
    FrameTooLarge,
    Protocol(ProtocolError),
}

impl std::fmt::Display for StdioError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "stdio I/O failed: {error}"),
            Self::EmptyFrame => f.write_str("empty JSON-RPC frame"),
            Self::FrameTooLarge => f.write_str("JSON-RPC frame exceeds limit"),
            Self::Protocol(error) => write!(f, "protocol failed: {error}"),
        }
    }
}

impl std::error::Error for StdioError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

/// Serve newline-delimited requests through the synchronous backend.
pub fn serve<R: BufRead, W: Write, B: ProtocolBackend + ?Sized>(
    reader: &mut R,
    writer: &mut W,
    session: &mut ProtocolSession,
    backend: &mut B,
) -> Result<(), StdioError> {
    serve_frames(reader, writer, |frame| {
        vexel_protocol::handle(session, frame, Some(backend)).map_err(StdioError::Protocol)
    })
}

/// Serve requests through the protocol's accepted async-backend route.
/// Async work is dispatched according to protocol semantics; this transport
/// does not spawn tasks or introduce an executor.
pub fn serve_async<R: BufRead, W: Write, B: ProtocolBackend + ?Sized, A: AsyncBackend + ?Sized>(
    reader: &mut R,
    writer: &mut W,
    session: &mut ProtocolSession,
    backend: &mut B,
    async_backend: &mut A,
) -> Result<(), StdioError> {
    serve_frames(reader, writer, |frame| {
        vexel_protocol::handle_async(session, frame, Some(backend), Some(async_backend))
            .map_err(StdioError::Protocol)
    })
}

fn serve_frames<R: BufRead, W: Write, F>(
    reader: &mut R,
    writer: &mut W,
    mut dispatch: F,
) -> Result<(), StdioError>
where
    F: FnMut(&[u8]) -> Result<Vec<u8>, StdioError>,
{
    let mut frame = Vec::new();
    let mut pending_cr = false;
    loop {
        let available = reader.fill_buf().map_err(StdioError::Io)?;
        if available.is_empty() {
            if pending_cr {
                return Err(StdioError::FrameTooLarge);
            }
            if frame.is_empty() {
                return Ok(());
            }
            process_frame(&mut frame, writer, &mut dispatch)?;
            return Ok(());
        }
        let mut consumed = 0;
        let mut completed = false;
        for &byte in available {
            if pending_cr {
                if byte != b'\n' {
                    return Err(StdioError::FrameTooLarge);
                }
                pending_cr = false;
                consumed += 1;
                completed = true;
                break;
            }
            consumed += 1;
            match byte {
                b'\n' => {
                    completed = true;
                    break;
                }
                b'\r' if frame.len() == MAX_BYTES => pending_cr = true,
                _ => {
                    if frame.len() == MAX_BYTES {
                        return Err(StdioError::FrameTooLarge);
                    }
                    frame
                        .try_reserve(1)
                        .map_err(|_| StdioError::FrameTooLarge)?;
                    frame.push(byte);
                }
            }
        }
        reader.consume(consumed);
        if completed {
            process_frame(&mut frame, writer, &mut dispatch)?;
        }
    }
}

fn process_frame<W: Write, F>(
    frame: &mut Vec<u8>,
    writer: &mut W,
    dispatch: &mut F,
) -> Result<(), StdioError>
where
    F: FnMut(&[u8]) -> Result<Vec<u8>, StdioError>,
{
    if frame.last() == Some(&b'\r') {
        frame.pop();
    }
    if frame.is_empty() {
        return Err(StdioError::EmptyFrame);
    }
    let response = dispatch(frame)?;
    frame.clear();
    if !response.is_empty() {
        writer.write_all(&response).map_err(StdioError::Io)?;
        writer.write_all(b"\n").map_err(StdioError::Io)?;
        writer.flush().map_err(StdioError::Io)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Read};
    use vexel_json::{JsonDocument, MAX_BYTES};
    use vexel_protocol::{AsyncDisposition, ToolResponse};

    #[derive(Default)]
    struct Backend {
        calls: usize,
    }
    impl ProtocolBackend for Backend {
        fn call_tool(&mut self, _: &JsonDocument<'_>, _: usize, _: &str) -> Option<ToolResponse> {
            self.calls += 1;
            Some(ToolResponse {
                structured_json: b"{}".to_vec(),
                content_json: Vec::new(),
                is_error: false,
            })
        }
    }
    #[derive(Default)]
    struct Async {
        calls: usize,
    }
    impl AsyncBackend for Async {
        fn tasks_available(&self) -> bool {
            false
        }
        fn call_tool(
            &mut self,
            _: &JsonDocument<'_>,
            _: usize,
            _: &str,
            _: bool,
            _: bool,
        ) -> (AsyncDisposition, Vec<u8>) {
            self.calls += 1;
            (AsyncDisposition::Unhandled, Vec::new())
        }
        fn get_task(&mut self, _: &JsonDocument<'_>, _: &str) -> (AsyncDisposition, Vec<u8>) {
            (AsyncDisposition::Failed, Vec::new())
        }
        fn cancel_task(&mut self, _: &JsonDocument<'_>, _: &str) -> (AsyncDisposition, Vec<u8>) {
            (AsyncDisposition::Failed, Vec::new())
        }
    }
    #[derive(Default)]
    struct FlushWriter {
        bytes: Vec<u8>,
        flushes: usize,
        fail: bool,
    }
    impl Write for FlushWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.fail {
                return Err(io::Error::other("write failure"));
            }
            let n = bytes.len().min(2);
            self.bytes.extend_from_slice(&bytes[..n]);
            Ok(n)
        }
        fn flush(&mut self) -> io::Result<()> {
            self.flushes += 1;
            if self.fail {
                Err(io::Error::other("flush failure"))
            } else {
                Ok(())
            }
        }
    }
    fn parsed_lines(bytes: &[u8]) -> Vec<JsonDocument<'_>> {
        bytes
            .split(|&b| b == b'\n')
            .filter(|line| !line.is_empty())
            .map(|line| JsonDocument::parse(line).unwrap())
            .collect()
    }
    const PING: &[u8] = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}";

    #[test]
    fn sync_multiple_crlf_eof_and_notification() {
        let input = [
            PING,
            b"\r\n",
            b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n",
            PING,
        ]
        .concat();
        let mut reader = Cursor::new(input);
        let mut writer = FlushWriter::default();
        serve(
            &mut reader,
            &mut writer,
            &mut ProtocolSession::default(),
            &mut Backend::default(),
        )
        .unwrap();
        let docs = parsed_lines(&writer.bytes);
        assert_eq!(docs.len(), 2);
        assert_eq!(writer.flushes, 2);
        assert_eq!(writer.bytes, b"{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n{\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n");
    }
    #[test]
    fn async_route_calls_actual_protocol_async_handler() {
        let initialize = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25"}}"#;
        let tool_call = br#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"document_list","arguments":{}}}"#;
        let mut reader = Cursor::new(
            [
                initialize.as_slice(),
                b"\n",
                br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
                b"\n",
                tool_call.as_slice(),
                b"\n",
            ]
            .concat(),
        );
        let mut writer = FlushWriter::default();
        let mut backend = Backend::default();
        let mut asynchronous = Async::default();
        serve_async(
            &mut reader,
            &mut writer,
            &mut ProtocolSession::default(),
            &mut backend,
            &mut asynchronous,
        )
        .unwrap();
        assert_eq!(parsed_lines(&writer.bytes).len(), 2);
        assert_eq!(asynchronous.calls, 1);
        assert_eq!(backend.calls, 1);
    }

    #[test]
    fn maximum_payload_with_crlf_is_accepted() {
        let mut payload = PING.to_vec();
        payload.resize(MAX_BYTES, b' ');
        let input = [payload, b"\r\n".to_vec()].concat();
        let mut reader = Cursor::new(input);
        let mut writer = FlushWriter::default();
        serve(
            &mut reader,
            &mut writer,
            &mut ProtocolSession::default(),
            &mut Backend::default(),
        )
        .unwrap();
        assert_eq!(parsed_lines(&writer.bytes).len(), 1);
        assert_eq!(writer.flushes, 1);
    }

    #[test]
    fn maximum_payload_crlf_split_across_reader_chunks_is_accepted() {
        struct SplitCrlfRead(Cursor<Vec<u8>>);
        impl Read for SplitCrlfRead {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                let position = self.0.position() as usize;
                let limit = if position < MAX_BYTES {
                    MAX_BYTES - position
                } else {
                    1
                };
                let size = out.len().min(limit);
                self.0.read(&mut out[..size])
            }
        }

        let mut payload = PING.to_vec();
        payload.resize(MAX_BYTES, b' ');
        let input = [payload, b"\r\n".to_vec()].concat();
        let source = SplitCrlfRead(Cursor::new(input));
        let mut reader = io::BufReader::with_capacity(97, source);
        let mut writer = FlushWriter::default();
        serve(
            &mut reader,
            &mut writer,
            &mut ProtocolSession::default(),
            &mut Backend::default(),
        )
        .unwrap();
        assert_eq!(parsed_lines(&writer.bytes).len(), 1);
        assert_eq!(writer.flushes, 1);
    }
    #[test]
    fn malformed_json_gets_protocol_parse_error() {
        let mut reader = Cursor::new(b"{bad}\n".to_vec());
        let mut writer = FlushWriter::default();
        serve(
            &mut reader,
            &mut writer,
            &mut ProtocolSession::default(),
            &mut Backend::default(),
        )
        .unwrap();
        assert!(writer.bytes.windows(11).any(|w| w == b"parse error"));
        assert_eq!(parsed_lines(&writer.bytes).len(), 1);
    }
    #[test]
    fn empty_and_oversized_frames_rejected_before_dispatch() {
        for input in [b"\n".to_vec(), vec![b'x'; MAX_BYTES + 1]] {
            let mut reader = Cursor::new(input);
            let mut writer = FlushWriter::default();
            let mut backend = Backend::default();
            let result = serve(
                &mut reader,
                &mut writer,
                &mut ProtocolSession::default(),
                &mut backend,
            );
            assert!(matches!(
                result,
                Err(StdioError::EmptyFrame | StdioError::FrameTooLarge)
            ));
            assert_eq!(backend.calls, 0);
            assert!(writer.bytes.is_empty());
        }
    }
    #[test]
    fn partial_reads_writes_and_reader_failure() {
        struct Chunks(Cursor<Vec<u8>>);
        impl Read for Chunks {
            fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
                let size = out.len().min(3);
                self.0.read(&mut out[..size])
            }
        }
        let mut reader =
            io::BufReader::with_capacity(3, Chunks(Cursor::new([PING, b"\n"].concat())));
        let mut writer = FlushWriter::default();
        serve(
            &mut reader,
            &mut writer,
            &mut ProtocolSession::default(),
            &mut Backend::default(),
        )
        .unwrap();
        assert_eq!(parsed_lines(&writer.bytes).len(), 1);
        struct BadReader;
        impl Read for BadReader {
            fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
                Err(io::Error::other("reader failure"))
            }
        }
        let mut reader = io::BufReader::new(BadReader);
        let mut writer = FlushWriter::default();
        assert!(matches!(
            serve(
                &mut reader,
                &mut writer,
                &mut ProtocolSession::default(),
                &mut Backend::default()
            ),
            Err(StdioError::Io(_))
        ));
        assert!(writer.bytes.is_empty());
    }
    #[test]
    fn writer_failure_propagates_after_protocol_response() {
        let mut reader = Cursor::new([PING, b"\n"].concat());
        let mut writer = FlushWriter {
            fail: true,
            ..Default::default()
        };
        assert!(matches!(
            serve(
                &mut reader,
                &mut writer,
                &mut ProtocolSession::default(),
                &mut Backend::default()
            ),
            Err(StdioError::Io(_))
        ));
    }
}
