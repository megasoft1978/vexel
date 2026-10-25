//! MCP stdio worker for the shell: `--mcp-preview` runs the shell (see
//! `session::McpHost`) and only `start_server` feeds it requests.
use crate::preview::{LiveBackend, UiRequest};
use std::{
    io::{BufReader, BufWriter, Write},
    sync::mpsc::SyncSender,
    thread,
};

pub fn start_server(requests: SyncSender<UiRequest>) -> Result<(), &'static str> {
    thread::Builder::new()
        .name("vexel-mcp-stdio".to_owned())
        .spawn(move || {
            let mut input = BufReader::new(std::io::stdin().lock());
            let mut output = BufWriter::new(std::io::stdout().lock());
            let mut session = vexel_protocol::ProtocolSession::live_preview();
            let mut backend = LiveBackend::new(requests);
            if let Err(error) =
                vexel_mcp_stdio::serve(&mut input, &mut output, &mut session, &mut backend)
            {
                eprintln!("Vexel MCP preview stopped: {error}");
            }
            // Flush before the request channel closes: its disconnect is what
            // tells the shell to exit after stdin EOF.
            let _ = output.flush();
            drop(backend);
        })
        .map(|_| ())
        .map_err(|_| "could not start the MCP stdio worker")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preview::{self, Operation, PreviewState};
    use std::sync::mpsc;
    use vexel_json::JsonDocument;
    use vexel_protocol::ProtocolBackend;
    #[test]
    fn backend_call_waits_until_the_ui_reports_frame_presentation() {
        let line = br##"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"element_add","arguments":{"documentId":"preview-document","parentId":"root","template":"rect","attributes":{"x":"1","y":"2","width":"3","height":"4","fill":"#00FF00"}}}}"##;
        let doc = JsonDocument::parse_owned(line).unwrap();
        let args = doc
            .object_get(doc.root(), b"params")
            .and_then(|p| doc.object_get(p, b"arguments"))
            .unwrap();
        let parsed = preview::parse_operation(&doc, args, "element_add");
        assert!(parsed.is_ok(), "preview request mapping failed: {parsed:?}");
        let (tx, rx) = mpsc::sync_channel(8);
        let worker = thread::spawn(move || {
            let mut backend = LiveBackend::new(tx);
            backend.call_tool(&doc, args, "element_add").unwrap()
        });
        let request = rx.recv().unwrap();
        assert!(matches!(&request.operation, Operation::Add(_)));
        assert!(!worker.is_finished(), "the tool must await its UI reply");
        let blank = PreviewState::new().unwrap();
        let staged = blank.stage(&request.operation).unwrap().0;
        assert_eq!(
            blank.shapes().len(),
            0,
            "staged edits are not yet committed"
        );
        let raster = preview::rasterize(staged.shapes()).unwrap();
        assert!(raster.iter().any(|&byte| byte != 255));
        request
            .reply
            .send(Ok(format!(
                "{{\"ok\":true,\"shapeCount\":{},\"elementId\":\"{}\"}}",
                staged.shapes().len(),
                staged.element_ids()[0]
            )))
            .unwrap();
        let response = worker.join().unwrap();
        assert!(!response.is_error);
        assert!(
            String::from_utf8(response.structured_json)
                .unwrap()
                .contains("shapeCount")
        );
    }
}
