//! Headless MCP rig: synthetic JSON-RPC `tools/call` lines go through
//! `vexel_mcp_stdio::serve` with the app's `LiveBackend`; the test thread
//! plays the UI loop, applying each request to one `AppDocument` through an
//! `McpHost` and the shared `Driver`.
#![allow(dead_code)]

use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::Duration;
use vexel_app::animation::{driver::Driver, job::ExportJob, playback::MonotonicClock};
use vexel_app::preview::{LiveBackend, Operation, UiRequest};
use vexel_app::session::{AppDocument, McpGrants, McpHost, McpOutcome};
use vexel_json::JsonDocument;
use vexel_mcp_access::AccessPolicy;
use vexel_protocol::ProtocolSession;

/// A temporary directory removed on drop.
pub struct Dir(pub PathBuf);

impl Dir {
    pub fn new() -> Self {
        static N: AtomicU32 = AtomicU32::new(0);
        let path = std::env::temp_dir().join(format!(
            "vexel-mcp-tools-{}-{}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }
    pub fn file(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }
    pub fn sub(&self, name: &str) -> PathBuf {
        let path = self.0.join(name);
        fs::create_dir_all(&path).unwrap();
        path
    }
    pub fn text(&self, name: &str) -> String {
        self.file(name).to_str().unwrap().to_owned()
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn fixture(name: &str) -> PathBuf {
    PathBuf::from(format!(
        "{}/../../fixtures/v1/mcp-tools/{name}",
        env!("CARGO_MANIFEST_DIR")
    ))
}

/// A policy that grants exactly `dirs` (no environment, no config file).
pub fn policy_for(dirs: &[&Path]) -> AccessPolicy {
    McpGrants::from_dirs(dirs.iter().copied(), None)
        .unwrap()
        .policy_with(None, None)
}

/// A policy with no grants at all.
pub fn no_grants() -> AccessPolicy {
    McpGrants::none().policy_with(None, None)
}

/// One tool reply.
#[derive(Debug)]
pub struct Reply {
    pub is_error: bool,
    /// The raw `structuredContent` JSON.
    pub body: String,
}

impl Reply {
    /// The typed error string of a failed call.
    pub fn error(&self) -> &str {
        assert!(self.is_error, "expected an error, got {}", self.body);
        self.body
            .strip_prefix("{\"error\":\"")
            .and_then(|rest| rest.strip_suffix("\"}"))
            .unwrap_or(&self.body)
    }
    pub fn ok(&self) -> &str {
        assert!(!self.is_error, "expected success, got {}", self.body);
        &self.body
    }
}

/// What a rejected call must not have changed.
#[derive(Debug, PartialEq, Eq)]
pub struct State {
    pub source: Vec<u8>,
    pub history: usize,
    pub revision: u64,
    pub dirty: bool,
}

pub struct Rig {
    pub app: AppDocument,
    pub host: McpHost,
    pub driver: Driver<MonotonicClock>,
    session: ProtocolSession,
    backend: LiveBackend,
    requests: Receiver<UiRequest>,
    next_id: u64,
}

impl Rig {
    pub fn new(policy: AccessPolicy) -> Self {
        Self::with_document(AppDocument::new(512, 512).unwrap(), policy)
    }

    /// A rig whose startup file is opened through `Operation::Open`, as
    /// `--mcp-preview <file>` does; a refusal is returned, not hidden.
    pub fn with_open(policy: AccessPolicy, path: &Path) -> Result<Self, &'static str> {
        let mut rig = Self::new(policy);
        let operation = Operation::Open {
            path: path.to_str().unwrap().to_owned(),
        };
        let Self {
            app, host, driver, ..
        } = &mut rig;
        host.handle_with(app, operation, driver)?;
        Ok(rig)
    }

    pub fn with_document(app: AppDocument, policy: AccessPolicy) -> Self {
        let (tx, requests) = mpsc::sync_channel(8);
        let mut rig = Self {
            app,
            host: McpHost::with_policy(policy),
            driver: Driver::new(MonotonicClock::new()),
            session: ProtocolSession::live_preview(),
            backend: LiveBackend::new(tx),
            requests,
            next_id: 1,
        };
        let init = r#"{"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
        rig.line(init);
        rig.line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);
        rig
    }

    pub fn state(&self) -> State {
        State {
            source: self.app.source_bytes().unwrap(),
            history: self.app.document().history_count(),
            revision: self.app.revision(),
            dirty: self.app.is_dirty(),
        }
    }

    /// Sends one JSON-RPC line and serves the UI requests it causes.
    pub fn line(&mut self, line: &str) -> String {
        let Self {
            app,
            host,
            driver,
            session,
            backend,
            requests,
            ..
        } = self;
        let mut input = Cursor::new(format!("{line}\n").into_bytes());
        let mut output = Vec::new();
        std::thread::scope(|scope| {
            let server = scope.spawn(|| {
                vexel_mcp_stdio::serve(&mut input, &mut output, session, backend).unwrap();
            });
            loop {
                match requests.recv_timeout(Duration::from_micros(100)) {
                    Ok(UiRequest { operation, reply }) => {
                        match host.handle_with(app, operation, driver) {
                            Ok(McpOutcome::Reply(json)) => {
                                let _ = reply.send(Ok(json));
                            }
                            Ok(McpOutcome::Export {
                                source,
                                path,
                                fps,
                                duration,
                            }) => {
                                let mut job = if source.starts_with(b"VEXEL_NATIVE_GIF\0") {
                                    ExportJob::spawn_native_packed(
                                        source,
                                        host.policy().clone(),
                                        path.into(),
                                        reply,
                                    )
                                } else {
                                    ExportJob::spawn(
                                        source,
                                        host.policy().clone(),
                                        path.into(),
                                        fps,
                                        duration,
                                        reply,
                                    )
                                }
                                .unwrap();
                                while job.poll().is_none() {
                                    std::thread::sleep(Duration::from_millis(2));
                                }
                            }
                            Err(message) => {
                                let _ = reply.send(Err(message));
                            }
                        }
                    }
                    Err(RecvTimeoutError::Timeout) if server.is_finished() => break,
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => break,
                }
            }
        });
        String::from_utf8(output).unwrap()
    }

    /// `tools/call` with a JSON `arguments` object text.
    pub fn tool(&mut self, name: &str, arguments: &str) -> Reply {
        self.next_id += 1;
        let line = format!(
            r#"{{"jsonrpc":"2.0","id":{},"method":"tools/call","params":{{"name":"{name}","arguments":{arguments}}}}}"#,
            self.next_id
        );
        let response = self.line(&line);
        parse_reply(&response)
    }

    /// A tool that takes `documentId` plus extra JSON members.
    pub fn doc_tool(&mut self, name: &str, extra: &str) -> Reply {
        let extra = extra.trim_start_matches(',');
        let comma = if extra.is_empty() { "" } else { "," };
        self.tool(
            name,
            &format!(r#"{{"documentId":"preview-document"{comma}{extra}}}"#),
        )
    }

    /// Calls a preview path tool whose schema requires `documentId` plus args.
    pub fn preview_tool(&mut self, name: &str, arguments: &str) -> Reply {
        let object = if arguments.trim_start().starts_with('{') {
            arguments.to_owned()
        } else {
            let arguments = arguments.trim_start_matches(',');
            let comma = if arguments.is_empty() { "" } else { "," };
            format!(r#"{{"documentId":"preview-document"{comma}{arguments}}}"#)
        };
        self.tool(name, &object)
    }

    pub fn list_tools(&mut self) -> Vec<String> {
        let out = self.line(r#"{"jsonrpc":"2.0","id":99,"method":"tools/list","params":{}}"#);
        let doc = JsonDocument::parse(out.as_bytes()).unwrap();
        let result = doc.object_get(doc.root(), b"result").unwrap();
        let tools = doc.object_get(result, b"tools").unwrap();
        let mut names = Vec::new();
        let mut item = doc.token(tools).unwrap().first_child;
        while let Some(at) = item {
            let name = doc.object_get(at, b"name").unwrap();
            names.push(String::from_utf8(doc.decoded_string(name).unwrap()).unwrap());
            item = doc.token(at).unwrap().next_sibling;
        }
        names
    }

    /// Adds a rectangle through MCP and returns its element id.
    pub fn add_rect(&mut self, x: u32, y: u32, fill: &str) -> String {
        let reply = self.doc_tool(
            "element_add",
            &format!(
                r#""parentId":"root","template":"rect","attributes":{{"x":"{x}","y":"{y}","width":"40","height":"30","fill":"{fill}"}}"#
            ),
        );
        element_id(reply.ok())
    }
}

pub fn element_id(body: &str) -> String {
    let rest = body.split("\"elementId\":\"").nth(1).expect(body);
    rest[..rest.find('"').unwrap()].to_owned()
}

fn parse_reply(response: &str) -> Reply {
    let doc = JsonDocument::parse(response.as_bytes()).unwrap();
    let result = doc
        .object_get(doc.root(), b"result")
        .unwrap_or_else(|| panic!("no result: {response}"));
    let is_error = doc
        .object_get(result, b"isError")
        .and_then(|t| doc.raw_token(t))
        == Some(b"true".as_slice());
    let body = doc
        .object_get(result, b"structuredContent")
        .and_then(|t| doc.raw_token(t))
        .map(|raw| String::from_utf8_lossy(raw).into_owned())
        .unwrap_or_default();
    Reply { is_error, body }
}
