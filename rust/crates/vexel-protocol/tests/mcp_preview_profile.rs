use vexel_json::JsonDocument;
use vexel_protocol::{ProtocolBackend, ProtocolSession, ToolResponse, handle, tool_schemas};

#[derive(Default)]
struct Backend {
    calls: Vec<String>,
}
impl ProtocolBackend for Backend {
    fn call_tool(
        &mut self,
        _request: &JsonDocument<'_>,
        _arguments: usize,
        name: &str,
    ) -> Option<ToolResponse> {
        self.calls.push(name.to_owned());
        Some(ToolResponse {
            structured_json: b"{}".to_vec(),
            content_json: Vec::new(),
            is_error: false,
        })
    }
}
fn call(session: &mut ProtocolSession, backend: &mut Backend, request: &str) -> Vec<u8> {
    handle(session, request.as_bytes(), Some(backend)).unwrap()
}
fn initialize(session: &mut ProtocolSession, backend: &mut Backend) {
    let req = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;
    let _ = call(session, backend, req);
    let _ = call(
        session,
        backend,
        r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    );
}
#[test]
fn live_preview_accepts_shipping_codex_handshake_version() {
    let mut preview = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    let request = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"Codex","version":"0.162.1"}}}"#;
    let response = call(&mut preview, &mut backend, request);
    assert!(
        String::from_utf8_lossy(&response).contains("\"protocolVersion\":\"2025-06-18\""),
        "unexpected initialize response: {}",
        String::from_utf8_lossy(&response)
    );
    assert_eq!(preview.protocol_version(), Some("2025-06-18"));

    let mut full = ProtocolSession::default();
    let response = call(&mut full, &mut backend, request);
    assert!(String::from_utf8_lossy(&response).contains("\"code\":-32004"));
    assert_eq!(full.protocol_version(), None);
}

#[test]
fn live_preview_advertises_only_supported_tools_and_narrow_shapes() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    let listing = call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    let full = tool_schemas().unwrap();
    let doc = JsonDocument::parse(&listing).unwrap();
    let result = doc.object_get(doc.root(), b"result").unwrap();
    let tools = doc.object_get(result, b"tools").unwrap();
    // The 9 original preview tools plus the 6 routed by WO-RUST-V1-MCP-TOOLS-01
    // (document_open, attribute_set, element_delete, save, save_as, export_svg)
    // plus render_png added by WO-RUST-V1-PNG-01, plus the 11 native_timeline_*
    // tools routed by the V1 native MCP slice, plus native_timeline_set_duration,
    // plus element_reparent and batch, plus element_add_fragment and use_add,
    // in declaration order.
    assert_eq!(doc.token(tools).unwrap().child_count, 66);
    assert!(String::from_utf8_lossy(&listing).contains("render_png"));
    for (idx, name) in [
        "document_open",
        "document_list",
        "document_info",
        "attribute_set",
        "element_add",
        "element_delete",
        "undo",
        "redo",
        "save",
        "save_as",
        "render_png",
        "animation_list",
        "animation_add",
        "animation_control",
        "native_timeline_get",
        "native_timeline_add_track",
        "native_timeline_add_drawing_asset",
        "native_timeline_add_property_track",
        "native_timeline_set_loop",
        "native_timeline_set_property_key",
        "native_timeline_set_cel",
        "native_timeline_remove_cel",
        "native_timeline_move_item",
        "native_timeline_undo",
        "native_timeline_redo",
        "export_animation",
        "export_svg",
    ]
    .iter()
    .enumerate()
    {
        // Tools added later (document, paint, query, timeline edit) interleave
        // with the original ones, so presence rather than position is checked.
        let mut item = doc.token(tools).unwrap().first_child;
        let mut found = false;
        while let Some(at) = item {
            found |= doc.string_equals(doc.object_get(at, b"name").unwrap(), name.as_bytes());
            item = doc.token(at).unwrap().next_sibling;
        }
        assert!(found, "missing {name} (expected near position {idx})");
    }
    let full_doc = JsonDocument::parse(&full).unwrap();
    let full_add = full_doc
        .token(full_doc.root())
        .unwrap()
        .first_child
        .unwrap();
    let full_add = (0..10).fold(full_add, |at, _| {
        full_doc.token(at).unwrap().next_sibling.unwrap()
    });
    assert!(
        String::from_utf8_lossy(
            &full_doc
                .raw_token(full_doc.object_get(full_add, b"inputSchema").unwrap())
                .unwrap()
        )
        .contains("circle")
    );
    assert!(String::from_utf8_lossy(&listing).contains(
        "\"template\":{\"type\":\"string\",\"enum\":[\"rect\",\"ellipse\",\"line\",\"path\"]}"
    ));
    assert!(
        String::from_utf8_lossy(&listing)
            .contains("Add a rectangle, ellipse, line, or path to the live preview document.")
    );
    assert!(
        String::from_utf8_lossy(&listing)
            .contains("\"required\":[\"documentId\",\"parentId\",\"template\",\"attributes\"]")
    );
    assert!(!String::from_utf8_lossy(&listing).contains("group, or text template"));
    assert!(!String::from_utf8_lossy(&listing).contains("resources"));
    assert!(!String::from_utf8_lossy(&listing).contains("prompts"));
    assert!(String::from_utf8_lossy(&listing).contains("animation_add"));
    assert!(String::from_utf8_lossy(&listing).contains("animation_control"));
    assert!(String::from_utf8_lossy(&listing).contains("\"maximum\":10000"));
    assert!(String::from_utf8_lossy(&listing).contains("export_animation"));
}

#[test]
fn preview_export_animation_validates_gif_fps_duration_and_path() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    let request = |id: u32, args: &str| {
        format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"export_animation","arguments":{{"documentId":"d",{args}}}}}}}"#
        )
    };
    let valid = request(
        2,
        r#""path":"/tmp/a.gif","format":"gif","fps":10,"duration":1.5"#,
    );
    assert!(
        String::from_utf8_lossy(&call(&mut session, &mut backend, &valid))
            .contains("\"isError\":false")
    );
    for (id, args) in [
        (
            3,
            r#""path":"/tmp/a.gif","format":"webp","fps":10,"duration":1"#,
        ),
        (
            4,
            r#""path":"/tmp/a.gif","format":"gif","fps":31,"duration":1"#,
        ),
        (
            5,
            r#""path":"/tmp/a.gif","format":"gif","fps":0,"duration":1"#,
        ),
        (
            6,
            r#""path":"/tmp/a.gif","format":"gif","fps":10,"duration":10.5"#,
        ),
        (
            7,
            r#""path":"/tmp/a.gif","format":"gif","fps":10,"duration":0"#,
        ),
        (8, r#""path":"","format":"gif","fps":10,"duration":1"#),
        (9, r#""format":"gif","fps":10,"duration":1"#),
    ] {
        let out = call(&mut session, &mut backend, &request(id, args));
        assert!(
            String::from_utf8_lossy(&out).contains("\"isError\":true"),
            "{args}"
        );
    }
    assert_eq!(backend.calls, ["export_animation"]);
}

#[test]
fn preview_export_animation_accepts_video_without_fps_or_duration() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    let request = |id: u32, args: &str| {
        format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"export_animation","arguments":{{"documentId":"d",{args}}}}}}}"#
        )
    };
    for (id, args, ok) in [
        (2, r#""path":"/tmp/a.mp4","format":"mp4""#, true),
        (
            3,
            r##""path":"/tmp/a.mov","format":"mov","codec":"hevc","quality":0.5,"width":64,"height":32,"loops":2,"fps":60,"background":"#fff""##,
            true,
        ),
        (
            4,
            r#""path":"/tmp/a.mp4","format":"mp4","bitrate":2500"#,
            true,
        ),
        (5, r#""format":"mp4""#, false),
        (6, r#""path":"/tmp/a.mp4","format":"avi""#, false),
        (7, r#""path":"/tmp/a.gif","format":"gif""#, false),
        (8, r#""path":"/tmp/a.mp4","format":"mp4","codec":7"#, false),
        (9, r#""path":"/tmp/a_####.png","format":"frames","fps":12"#, true),
        (10, r#""path":"/tmp/a.apng","format":"apng","loops":0,"width":8,"height":8"#, true),
    ] {
        let out = call(&mut session, &mut backend, &request(id, args));
        let text = String::from_utf8_lossy(&out);
        assert_eq!(text.contains("\"isError\":false"), ok, "{args}: {text}");
    }
    assert_eq!(backend.calls.len(), 5);
}

#[test]
fn preview_animation_tools_validate_bounded_fragments_and_controls() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    let valid = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"animation_add","arguments":{"documentId":"d","targetId":"shape-1","xml":"<animate attributeName=\"x\" values=\"0;10\" dur=\"1s\"/>"}}}"#;
    assert!(
        String::from_utf8_lossy(&call(&mut session, &mut backend, valid))
            .contains("\"isError\":false")
    );
    let invalid_fragment = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"animation_add","arguments":{"documentId":"d","targetId":"shape-1","xml":"<script/>"}}}"#;
    assert!(
        String::from_utf8_lossy(&call(&mut session, &mut backend, invalid_fragment))
            .contains("\"isError\":true")
    );
    let invalid_action = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"animation_control","arguments":{"documentId":"d","action":"restart"}}}"#;
    assert!(
        String::from_utf8_lossy(&call(&mut session, &mut backend, invalid_action))
            .contains("\"isError\":true")
    );
    let missing_seek_time = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"animation_control","arguments":{"documentId":"d","action":"seek"}}}"#;
    assert!(
        String::from_utf8_lossy(&call(&mut session, &mut backend, missing_seek_time))
            .contains("\"isError\":true")
    );
    let play = r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"animation_control","arguments":{"documentId":"d","action":"play"}}}"#;
    assert!(
        String::from_utf8_lossy(&call(&mut session, &mut backend, play))
            .contains("\"isError\":false")
    );
    assert_eq!(backend.calls, ["animation_add", "animation_control"]);
}
#[test]
fn live_preview_rejects_unadvertised_tools_and_shape_templates() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    for (id, name, template) in [
        ("2", "document_open", "rect"),
        ("3", "element_add", "circle"),
    ] {
        let request = format!(
            r#"{{"jsonrpc":"2.0","id":{id},"method":"tools/call","params":{{"name":"{name}","arguments":{{"documentId":"d","parentId":"root","template":"{template}"}}}}}}"#
        );
        let out = call(&mut session, &mut backend, &request);
        assert!(String::from_utf8_lossy(&out).contains("\"isError\":true"));
    }
    assert!(backend.calls.is_empty());
    let rect = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"element_add","arguments":{"documentId":"d","parentId":"root","template":"rect"}}}"#;
    let out = call(&mut session, &mut backend, rect);
    assert!(String::from_utf8_lossy(&out).contains("\"isError\":true"));
    assert!(backend.calls.is_empty());
    let rect = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"element_add","arguments":{"documentId":"d","parentId":"root","template":"rect","attributes":{}}}}"#;
    let out = call(&mut session, &mut backend, rect);
    assert!(String::from_utf8_lossy(&out).contains("\"isError\":false"));
    assert_eq!(backend.calls, ["element_add"]);
}
#[test]
fn default_profile_keeps_full_schema_resources_prompts_and_reset_profile() {
    let full_schema = tool_schemas().unwrap();
    assert!(!String::from_utf8_lossy(&full_schema).contains("animation_control"));
    let mut session = ProtocolSession::default();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    let tools = call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}"#,
    );
    assert!(String::from_utf8_lossy(&tools).contains("document_open"));
    assert!(String::from_utf8_lossy(&tools).contains("circle"));
    assert!(!String::from_utf8_lossy(&tools).contains("animation_control"));
    let control = call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"animation_control","arguments":{"documentId":"d","action":"play"}}}"#,
    );
    assert!(String::from_utf8_lossy(&control).contains("\"isError\":true"));
    assert!(String::from_utf8_lossy(&control).contains("unknown tool"));
    let resources = call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":3,"method":"resources/list","params":{}}"#,
    );
    let prompts = call(
        &mut session,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":4,"method":"prompts/list","params":{}}"#,
    );
    assert!(String::from_utf8_lossy(&resources).contains("vexel://documents/current/source"));
    assert!(String::from_utf8_lossy(&prompts).contains("inspect-document"));
    let mut preview = ProtocolSession::live_preview();
    preview.reset();
    initialize(&mut preview, &mut backend);
    let reset_listing = call(
        &mut preview,
        &mut backend,
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/list","params":{}}"#,
    );
    assert!(
        String::from_utf8_lossy(&reset_listing)
            .contains("\"enum\":[\"rect\",\"ellipse\",\"line\",\"path\"]")
    );
}

#[test]
fn preview_modern_discovery_omits_unavailable_capabilities() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    let discover = r#"{"jsonrpc":"2.0","id":7,"method":"server/discover","params":{"_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#;
    let output = call(&mut session, &mut backend, discover);
    let text = String::from_utf8_lossy(&output);
    assert!(text.contains("\"tools\":{"));
    assert!(!text.contains("io.modelcontextprotocol/tasks"));
    assert!(!text.contains("\"resources\""));
    assert!(!text.contains("\"prompts\""));
    assert!(text.contains("basic rectangles and ellipses"));
    assert!(!text.contains("Save As"));
    assert!(!text.contains("documents open in the Vexel app"));

    let tasks = r#"{"jsonrpc":"2.0","id":8,"method":"tasks/get","params":{"taskId":"preview-task","_meta":{"io.modelcontextprotocol/protocolVersion":"2026-07-28"}}}"#;
    let task_output = call(&mut session, &mut backend, tasks);
    let task_text = String::from_utf8_lossy(&task_output);
    assert!(task_text.contains("\"code\":-32601"));
    assert!(task_text.contains("method not found"));
}

#[test]
fn live_preview_forwards_routed_file_edit_and_line_path_calls() {
    let mut session = ProtocolSession::live_preview();
    let mut backend = Backend::default();
    initialize(&mut session, &mut backend);
    let requests = [
        r#"{"name":"save","arguments":{"documentId":"d"}}"#,
        r#"{"name":"save_as","arguments":{"documentId":"d","path":"/x/a.vexel"}}"#,
        r#"{"name":"document_open","arguments":{"path":"/x/a.vexel"}}"#,
        r##"{"name":"attribute_set","arguments":{"documentId":"d","elementId":"e","name":"fill","value":"#000000"}}"##,
        r#"{"name":"element_delete","arguments":{"documentId":"d","elementId":"e"}}"#,
        r#"{"name":"export_svg","arguments":{"documentId":"d"}}"#,
        r#"{"name":"element_add","arguments":{"documentId":"d","parentId":"root","template":"line","attributes":{}}}"#,
        r#"{"name":"element_add","arguments":{"documentId":"d","parentId":"root","template":"path","attributes":{}}}"#,
    ];
    for (id, params) in requests.iter().enumerate() {
        let request = format!(
            r#"{{"jsonrpc":"2.0","id":{},"method":"tools/call","params":{params}}}"#,
            id + 10
        );
        let out = call(&mut session, &mut backend, &request);
        assert!(
            String::from_utf8_lossy(&out).contains("\"isError\":false"),
            "{params}"
        );
    }
    assert_eq!(backend.calls.len(), requests.len());
    // render_png is routed since WO-RUST-V1-PNG-01, but a call without the required
    // path/width/height arguments is refused by validation before the backend.
    let png = r#"{"jsonrpc":"2.0","id":30,"method":"tools/call","params":{"name":"render_png","arguments":{"documentId":"d"}}}"#;
    let out = call(&mut session, &mut backend, png);
    let text = String::from_utf8_lossy(&out);
    assert!(text.contains("\"isError\":true"), "{text}");
    assert!(text.contains("invalid_arguments"), "{text}");
    assert_eq!(backend.calls.len(), requests.len());
}
