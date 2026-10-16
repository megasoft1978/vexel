use super::{ProtocolError, ToolSchema, append_string};
use vexel_json::{JsonBuffer, JsonDocument, JsonType, MAX_BYTES};

pub(super) const TOOLS: &[ToolSchema] = &[
    ToolSchema {
        name: "document_open",
        description: "Open a local SVG within a user-granted directory.",
        properties: "{\"path\":{\"type\":\"string\"}}",
        required: "path",
        fields: "path:string",
    },
    ToolSchema {
        name: "document_close",
        description: "Close a non-active MCP-opened document after checking for unsaved edits.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "document_list",
        description: "List SVG documents open in Vexel.",
        properties: "{}",
        required: "",
        fields: "",
    },
    ToolSchema {
        name: "document_info",
        description: "Read the current size, path, and edit state of an open SVG.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "tree_get",
        description: "Read a bounded page of the document element tree.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"cursor\":{\"type\":\"integer\",\"minimum\":0},\"limit\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":512}}",
        required: "documentId",
        fields: "documentId:string;cursor:integer;limit:integer",
    },
    ToolSchema {
        name: "element_get",
        description: "Read one element and its attributes by id or returned element handle.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string",
    },
    ToolSchema {
        name: "element_query",
        description: "Find elements by id, tag, or exact attribute value.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"id\":{\"type\":\"string\"},\"tag\":{\"type\":\"string\"},\"attribute\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"},\"cursor\":{\"type\":\"integer\",\"minimum\":0},\"limit\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":512}}",
        required: "documentId",
        fields: "documentId:string;id:string;tag:string;attribute:string;value:string;cursor:integer;limit:integer",
    },
    ToolSchema {
        name: "attribute_set",
        description: "Set an existing XML attribute through the undoable edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"}}",
        required: "documentId,elementId,name,value",
        fields: "documentId:string;elementId:string;name:string;value:string",
    },
    ToolSchema {
        name: "attribute_remove",
        description: "Remove an XML attribute through the undoable edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"}}",
        required: "documentId,elementId,name",
        fields: "documentId:string;elementId:string;name:string",
    },
    ToolSchema {
        name: "text_set",
        description: "Replace an element's text content through the undoable edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"text\":{\"type\":\"string\"}}",
        required: "documentId,elementId,text",
        fields: "documentId:string;elementId:string;text:string",
    },
    ToolSchema {
        name: "element_add",
        description: "Add a basic SVG shape, group, or text template as a child of an element.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"template\":{\"type\":\"string\",\"enum\":[\"rect\",\"circle\",\"ellipse\",\"line\",\"path\",\"g\",\"text\"]},\"attributes\":{\"type\":\"object\",\"additionalProperties\":{\"type\":\"string\"}}}",
        required: "documentId,parentId,template",
        fields: "documentId:string;parentId:string;template:string;attributes:object",
    },
    ToolSchema {
        name: "element_add_fragment",
        description: "Add one validated XML element fragment as a child of an element.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"xml\":{\"type\":\"string\",\"maxLength\":65536}}",
        required: "documentId,parentId,xml",
        fields: "documentId:string;parentId:string;xml:string",
    },
    ToolSchema {
        name: "element_delete",
        description: "Delete a non-root element through the undoable edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string",
    },
    ToolSchema {
        name: "element_duplicate",
        description: "Duplicate an element as its next sibling.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string",
    },
    ToolSchema {
        name: "element_reorder",
        description: "Move an element before or after a sibling element.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"targetId\":{\"type\":\"string\"},\"before\":{\"type\":\"boolean\"}}",
        required: "documentId,elementId,targetId,before",
        fields: "documentId:string;elementId:string;targetId:string;before:boolean",
    },
    ToolSchema {
        name: "element_group",
        description: "Wrap an element in an SVG group.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string",
    },
    ToolSchema {
        name: "element_ungroup",
        description: "Remove a group while retaining its children.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string",
    },
    ToolSchema {
        name: "transaction",
        description: "Apply a bounded edit batch atomically; failed batches are discarded.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"edits\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":64,\"items\":{\"type\":\"object\"}}}",
        required: "documentId,edits",
        fields: "documentId:string;edits:array",
    },
    ToolSchema {
        name: "undo",
        description: "Undo the latest document edit.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "redo",
        description: "Redo the next document edit.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "save",
        description: "Save the current document to its open path.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "save_as",
        description: "Save the current document atomically inside a granted directory.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"path\":{\"type\":\"string\"}}",
        required: "documentId,path",
        fields: "documentId:string;path:string",
    },
    ToolSchema {
        name: "render_png",
        description: "Render a bounded image of an open SVG region.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"region\":{\"type\":\"object\"},\"scale\":{\"type\":\"number\",\"minimum\":0.01,\"maximum\":16},\"time\":{\"type\":\"number\",\"minimum\":0,\"maximum\":3600}}",
        required: "documentId",
        fields: "documentId:string;region:object;scale:number;time:number",
    },
    ToolSchema {
        name: "animation_list",
        description: "List supported SMIL animation tracks, key values, spans, and diagnostics.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "animation_add",
        description: "Add one validated SMIL animation element to a target through the edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"targetId\":{\"type\":\"string\"},\"xml\":{\"type\":\"string\",\"maxLength\":65536}}",
        required: "documentId,targetId,xml",
        fields: "documentId:string;targetId:string;xml:string",
    },
    ToolSchema {
        name: "animation_control",
        description: "Control preview playback by playing, pausing, or seeking.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"action\":{\"type\":\"string\",\"enum\":[\"play\",\"pause\",\"seek\"]},\"timeMs\":{\"type\":\"number\",\"minimum\":0,\"maximum\":10000}}",
        required: "documentId,action",
        fields: "documentId:string;action:string;timeMs:number",
    },
    ToolSchema {
        name: "animation_set",
        description: "Set or add an attribute on one animation track through the edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"animationId\":{\"type\":\"string\"},\"name\":{\"type\":\"string\"},\"value\":{\"type\":\"string\"}}",
        required: "documentId,animationId,name,value",
        fields: "documentId:string;animationId:string;name:string;value:string",
    },
    ToolSchema {
        name: "animation_remove",
        description: "Remove one animation track through the edit engine.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"animationId\":{\"type\":\"string\"}}",
        required: "documentId,animationId",
        fields: "documentId:string;animationId:string",
    },
    ToolSchema {
        name: "native_timeline_get",
        description: "Read the active native-v1 project's timing, tracks, and drawing cels.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "native_timeline_add_track",
        description: "Add an undoable drawing track to the active native-v1 project.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"targetId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"assetName\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"name\":{\"type\":\"string\",\"maxLength\":256}}",
        required: "documentId,trackId,targetId,assetName",
        fields: "documentId:string;trackId:string;targetId:string;assetName:string;name:string",
    },
    ToolSchema {
        name: "native_timeline_add_drawing_asset",
        description: "Add a validated vector drawing asset to the active native-v1 project for use by drawing tracks and cels.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"assetName\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"svg\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":1048576}}",
        required: "documentId,assetName,svg",
        fields: "documentId:string;assetName:string;svg:string",
    },
    ToolSchema {
        name: "native_timeline_add_property_track",
        description: "Create an undoable native property track with its initial tick-zero value.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"targetId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"property\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"valueType\":{\"type\":\"string\",\"enum\":[\"scalar\",\"rgba\",\"transform2d\",\"boolean\"]},\"interpolation\":{\"type\":\"string\",\"enum\":[\"step\",\"linear\",\"cubic\"]},\"value\":{\"type\":[\"number\",\"boolean\",\"array\"]},\"name\":{\"type\":\"string\",\"maxLength\":256}}",
        required: "documentId,trackId,targetId,property,valueType,interpolation,value",
        fields: "documentId:string;trackId:string;targetId:string;property:string;valueType:string;interpolation:string;value:number|boolean|array;name:string",
    },
    ToolSchema {
        name: "native_timeline_set_loop",
        description: "Set the active native-v1 project's undoable playback mode.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"mode\":{\"type\":\"string\",\"enum\":[\"once\",\"loop\",\"ping_pong\"]}}",
        required: "documentId,mode",
        fields: "documentId:string;mode:string",
    },
    ToolSchema {
        name: "native_timeline_set_property_key",
        description: "Set or replace one undoable native property key at an integer tick.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"value\":{\"type\":[\"number\",\"boolean\",\"array\"]}}",
        required: "documentId,trackId,tick,value",
        fields: "documentId:string;trackId:string;tick:integer;value:number|boolean|array",
    },
    ToolSchema {
        name: "native_timeline_set_cel",
        description: "Set an undoable drawing exposure at a timeline tick.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"assetName\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"label\":{\"type\":\"string\",\"maxLength\":256}}",
        required: "documentId,trackId,tick,assetName",
        fields: "documentId:string;trackId:string;tick:integer;assetName:string;label:string",
    },
    ToolSchema {
        name: "native_timeline_remove_cel",
        description: "Remove an undoable drawing exposure from a timeline tick.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}}",
        required: "documentId,trackId,tick",
        fields: "documentId:string;trackId:string;tick:integer",
    },
    ToolSchema {
        name: "native_timeline_move_item",
        description: "Move an existing native drawing cel or property key to another integer tick.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"fromTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"toTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}}",
        required: "documentId,trackId,fromTick,toTick",
        fields: "documentId:string;trackId:string;fromTick:integer;toTick:integer",
    },
    ToolSchema {
        name: "native_timeline_undo",
        description: "Undo the latest native timeline edit in the active project.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "native_timeline_redo",
        description: "Redo the next native timeline edit in the active project.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "export_animation",
        description: "Export a bounded animation as a GIF or PNG frame sequence inside a granted directory.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"path\":{\"type\":\"string\"},\"format\":{\"type\":\"string\",\"enum\":[\"gif\",\"frames\"]},\"fps\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":30},\"duration\":{\"type\":\"number\",\"minimum\":0.01,\"maximum\":3600}}",
        required: "documentId,path,format,fps,duration",
        fields: "documentId:string;path:string;format:string;fps:integer;duration:number",
    },
    ToolSchema {
        name: "selection_get",
        description: "Read Vexel's current selection for a document.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "selection_set",
        description: "Select an element in Vexel by id or returned element handle.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string",
    },
    ToolSchema {
        name: "export_svg",
        description: "Return the current source-preserving SVG document text.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolValidationError {
    UnknownTool,
    UnexpectedArgument,
    WrongType,
    MissingRequired,
    InvalidLoopMode,
    InvalidTemplateAttributes,
    InvalidTransactionLength,
    InvalidArguments,
}

pub fn tool_schemas() -> Result<Vec<u8>, ProtocolError> {
    tool_schemas_for_preview(false)
}

pub(super) fn tool_schemas_for_preview(preview: bool) -> Result<Vec<u8>, ProtocolError> {
    let mut out = JsonBuffer::new(4096, MAX_BYTES).map_err(ProtocolError::Buffer)?;
    out.literal("[").map_err(ProtocolError::Buffer)?;
    let selected: Vec<_> = TOOLS
        .iter()
        .filter(|tool| {
            !(tool.name == "animation_control" && !preview)
                && (!preview
                    || matches!(
                        tool.name,
                        "document_list"
                            | "document_info"
                            | "element_add"
                            | "undo"
                            | "redo"
                            | "animation_list"
                            | "animation_add"
                            | "animation_control"
                            | "export_animation"
                            | "document_open"
                            | "attribute_set"
                            | "element_delete"
                            | "save"
                            | "save_as"
                            | "export_svg"
                    ))
        })
        .collect();
    for (index, tool) in selected.iter().enumerate() {
        if index != 0 {
            out.literal(",").map_err(ProtocolError::Buffer)?;
        }
        out.literal("{\"name\":").map_err(ProtocolError::Buffer)?;
        append_string(&mut out, tool.name.as_bytes())?;
        out.literal(",\"description\":")
            .map_err(ProtocolError::Buffer)?;
        let description = if preview && tool.name == "element_add" {
            "Add a rectangle, ellipse, line, or path to the live preview document."
        } else if preview && tool.name == "export_animation" {
            "Export the live preview loop as a GIF inside a granted directory."
        } else {
            tool.description
        };
        append_string(&mut out, description.as_bytes())?;
        out.literal(",\"inputSchema\":{\"type\":\"object\",\"properties\":")
            .map_err(ProtocolError::Buffer)?;
        if preview && tool.name == "element_add" {
            out.literal(r#"{"documentId":{"type":"string"},"parentId":{"type":"string"},"template":{"type":"string","enum":["rect","ellipse","line","path"]},"attributes":{"type":"object","additionalProperties":{"type":"string"}}}"#).map_err(ProtocolError::Buffer)?;
        } else {
            let properties = if preview && tool.name == "animation_add" {
                r#"{"documentId":{"type":"string"},"targetId":{"type":"string","minLength":1,"maxLength":128},"xml":{"type":"string","maxLength":4096,"pattern":"^<animate\\s[^>]*\\s/>$"}}"#
            } else if preview && tool.name == "animation_control" {
                r#"{"documentId":{"type":"string"},"action":{"type":"string","enum":["play","pause","seek"]},"timeMs":{"type":"number","minimum":0,"maximum":10000}}"#
            } else if preview && tool.name == "export_animation" {
                r#"{"documentId":{"type":"string"},"path":{"type":"string","minLength":1,"maxLength":1023},"format":{"type":"string","enum":["gif"]},"fps":{"type":"integer","minimum":1,"maximum":30},"duration":{"type":"number","minimum":0.01,"maximum":10}}"#
            } else {
                tool.properties
            };
            out.literal(properties).map_err(ProtocolError::Buffer)?;
        }
        out.literal(",\"required\":")
            .map_err(ProtocolError::Buffer)?;
        out.literal("[").map_err(ProtocolError::Buffer)?;
        let required = if preview && tool.name == "element_add" {
            "documentId,parentId,template,attributes"
        } else if preview && tool.name == "animation_control" {
            "documentId,action"
        } else {
            tool.required
        };
        for (required_index, field) in required.split(',').filter(|s| !s.is_empty()).enumerate() {
            if required_index != 0 {
                out.literal(",").map_err(ProtocolError::Buffer)?;
            }
            append_string(&mut out, field.as_bytes())?;
        }
        out.literal("],\"additionalProperties\":false}}")
            .map_err(ProtocolError::Buffer)?;
    }
    out.literal("]").map_err(ProtocolError::Buffer)?;
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(out.len())
        .map_err(|_| ProtocolError::Allocation)?;
    bytes.extend_from_slice(out.as_bytes());
    Ok(bytes)
}

pub fn validate_tool_arguments(
    doc: &JsonDocument<'_>,
    arguments: usize,
    tool_name: &str,
) -> Result<(), ToolValidationError> {
    let Some(args) = doc.token(arguments) else {
        return Err(ToolValidationError::InvalidArguments);
    };
    if args.kind != JsonType::Object {
        return Err(ToolValidationError::InvalidArguments);
    }
    let Some(tool) = TOOLS.iter().find(|t| t.name == tool_name) else {
        return Err(ToolValidationError::UnknownTool);
    };
    let mut key = args.first_child;
    while let Some(key_index) = key {
        let Some(value) = doc.token(key_index).and_then(|t| t.next_sibling) else {
            return Err(ToolValidationError::InvalidArguments);
        };
        let Some(key_bytes) = doc.decoded_string(key_index) else {
            return Err(ToolValidationError::InvalidArguments);
        };
        let Ok(field) = std::str::from_utf8(&key_bytes) else {
            return Err(ToolValidationError::UnexpectedArgument);
        };
        let Some(expected) = tool.fields.split(';').find_map(|field_spec| {
            let (name, kind) = field_spec.split_once(':')?;
            (name == field).then_some(kind)
        }) else {
            return Err(ToolValidationError::UnexpectedArgument);
        };
        if !expected
            .split('|')
            .any(|kind| type_matches(doc, value, kind))
        {
            return Err(ToolValidationError::WrongType);
        }
        if tool.name == "native_timeline_set_loop" && field == "mode" {
            let valid = [b"once".as_slice(), b"loop", b"ping_pong"]
                .iter()
                .any(|v| doc.string_equals(value, v));
            if !valid {
                return Err(ToolValidationError::InvalidLoopMode);
            }
        }
        if tool.name == "element_add" && field == "attributes" {
            let attrs = doc
                .token(value)
                .ok_or(ToolValidationError::InvalidArguments)?;
            let mut child = attrs.first_child;
            while let Some(k) = child {
                let Some(v) = doc.token(k).and_then(|t| t.next_sibling) else {
                    return Err(ToolValidationError::InvalidTemplateAttributes);
                };
                if doc.token(k).is_none_or(|t| t.kind != JsonType::String)
                    || doc.token(v).is_none_or(|t| t.kind != JsonType::String)
                {
                    return Err(ToolValidationError::InvalidTemplateAttributes);
                }
                child = doc.token(v).and_then(|t| t.next_sibling);
            }
        }
        if tool.name == "transaction" && field == "edits" {
            let count = doc.token(value).map_or(0, |t| t.child_count);
            if !(1..=64).contains(&count) {
                return Err(ToolValidationError::InvalidTransactionLength);
            }
        }
        key = doc.token(value).and_then(|t| t.next_sibling);
    }
    for required in tool.required.split(',').filter(|s| !s.is_empty()) {
        if doc.object_get(arguments, required.as_bytes()).is_none() {
            return Err(ToolValidationError::MissingRequired);
        }
    }
    Ok(())
}

fn type_matches(doc: &JsonDocument<'_>, token: usize, kind: &str) -> bool {
    let Some(item) = doc.token(token) else {
        return false;
    };
    match kind {
        "string" => item.kind == JsonType::String,
        "object" => item.kind == JsonType::Object,
        "array" => item.kind == JsonType::Array,
        "boolean" => matches!(item.kind, JsonType::True | JsonType::False),
        "number" => item.kind == JsonType::Number,
        "integer" => doc.uint(token).is_some(),
        _ => false,
    }
}
