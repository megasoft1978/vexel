use super::{ProtocolError, ToolSchema, append_string};
use vexel_json::{JsonBuffer, JsonDocument, JsonType, MAX_BYTES};

const RECT_KEYS: &[&[u8]] = &[b"x", b"y", b"width", b"height", b"fill"];
const ELLIPSE_KEYS: &[&[u8]] = &[b"cx", b"cy", b"rx", b"ry", b"fill"];
const LINE_KEYS: &[&[u8]] = &[b"x1", b"y1", b"x2", b"y2", b"stroke", b"stroke-width"];
const PATH_KEYS: &[&[u8]] = &[
    b"d",
    b"fill",
    b"stroke",
    b"stroke-width",
    b"stroke-linecap",
    b"stroke-linejoin",
    b"fill-rule",
];

pub fn element_add_keys(template: &str) -> Option<&'static [&'static [u8]]> {
    match template {
        "rect" => Some(RECT_KEYS),
        "ellipse" => Some(ELLIPSE_KEYS),
        "line" => Some(LINE_KEYS),
        "path" => Some(PATH_KEYS),
        _ => None,
    }
}

fn display_keys(keys: &[&[u8]]) -> String {
    keys.iter()
        .map(|key| std::str::from_utf8(key).expect("static attribute keys are UTF-8"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn element_add_description() -> String {
    format!(
        "Add a rectangle, ellipse, line, or path to the live preview document. parentId is \"root\" or the id of an existing group (see element_group), so nested hierarchies can be built. Attributes must be strings. Allowed attributes: rect: {}; ellipse: {}; line: {}; path: {}. Optional id gives the element an explicit id (1-128 characters [A-Za-z0-9_.-], starting with a letter or underscore; an id already in the document is rejected); opacity (0-1), transform ({{translateX, translateY, rotate, pivotX, pivotY, scaleX, scaleY, skewX}}, numbers as in element_set_transform) and gradientId (a gradient_add id, applied as fill, or as stroke with paint \"stroke\") are applied in the same undoable edit.",
        display_keys(RECT_KEYS),
        display_keys(ELLIPSE_KEYS),
        display_keys(LINE_KEYS),
        display_keys(PATH_KEYS),
    )
}

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
        description: "Set an existing XML attribute through the undoable edit engine. name \"id\" renames any element (groups included): value is the new id (1-64 characters [A-Za-z0-9_.-], starting with a letter or underscore, unused), href=\"#id\" and url(#id) references and native timeline tracks that target it follow, in one undo step.",
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
        fields: "documentId:string;parentId:string;template:string;attributes:object;id:string;opacity:number;transform:object;gradientId:string;paint:string",
    },
    ToolSchema {
        name: "element_add_fragment",
        description: "Add one validated XML element fragment (at most 1 MiB) as a child of an element.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\"},\"xml\":{\"type\":\"string\",\"maxLength\":1048576}}",
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
        description: "Wrap an element in an SVG group, or wrap several sibling elements (same parent) in one group with elementIds (1-1024 ids; give elementId or elementIds, not both). The group keeps their document order.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"elementIds\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":1024,\"items\":{\"type\":\"string\"}}}",
        required: "documentId",
        fields: "documentId:string;elementId:string;elementIds:array",
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
        fields: "documentId:string;region:object;scale:number;time:number;path:string;width:integer;height:integer",
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
        description: "Read the active native-v1 project's timing (ticksPerSecond, durationTicks, loop), tracks, and their items by tick (drawing cels with asset, property keys with value). The default timeline is 24 ticks per second and 48 ticks long, so valid ticks are integers 0-47 until native_timeline_set_duration changes it.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
    ToolSchema {
        name: "native_timeline_add_track",
        description: "Add an undoable drawing track that shows asset assetName at tick 0. The asset must already exist (add it first with native_timeline_add_drawing_asset). trackId: new unique id of 1-128 ASCII letters, digits, `_` or `-`. targetId: the id attribute of an existing document element, for example the elementId returned by element_add such as vexel-object-1. Example: {\"documentId\":\"preview-document\",\"trackId\":\"ball\",\"targetId\":\"vexel-object-1\",\"assetName\":\"ball-high\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"targetId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"assetName\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"name\":{\"type\":\"string\",\"maxLength\":256}}",
        required: "documentId,trackId,targetId,assetName",
        fields: "documentId:string;trackId:string;targetId:string;assetName:string;name:string",
    },
    ToolSchema {
        name: "native_timeline_add_drawing_asset",
        description: "Add a validated SVG drawing asset (one cel image) for use by drawing tracks and cels. svg must be one complete SVG document whose root <svg> has width, height and viewBox equal to the document size: width=W height=H viewBox=\"0 0 W H\" (read W,H from document_info; the live preview is 512x512). It must not contain <script>, DOCTYPE, on* attributes, xml:base or external references; href and url() may only point to #ids defined in the same svg. assetName: unique, 1-128 ASCII letters, digits, `.`, `_` or `-`. Example svg: <svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\"><circle cx=\"256\" cy=\"140\" r=\"32\" fill=\"#e74c3c\"/></svg>",
        properties: "{\"documentId\":{\"type\":\"string\"},\"assetName\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"svg\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":1048576}}",
        required: "documentId,assetName,svg",
        fields: "documentId:string;assetName:string;svg:string",
    },
    ToolSchema {
        name: "native_timeline_add_property_track",
        description: "Create an undoable native property track with its initial tick-zero value. targetId: id attribute of an existing element, e.g. vexel-object-1. Allowed property/valueType pairs: x, y, width, height, cx, cy, r, rx, ry, opacity, fill-opacity, stroke-opacity, stroke-width, stroke-dashoffset with scalar (a number); fill or stroke with rgba (array of 4 numbers 0-1); transform with transform2d (array of 6 numbers) or trs (array [tx,ty,rotationDeg,sx,sy,pivotX,pivotY] with optional skewXDeg, or an object such as {\"translate\":[0,-3],\"rotate\":15,\"scale\":[1,1],\"pivot\":[250,400]}; omitted parts are identity); visibility with boolean (interpolation step only); d (path elements) or points (polygon/polyline) with path (a path data string: every key must have the same commands after normalising to absolute M/L/C/Q/Z, arcs unsupported; points are blended per coordinate); filter primitives (targetId = the primitive element id) stdDeviation (feGaussianBlur/feDropShadow, 0-32), dx and dy (feOffset/feDropShadow), flood-opacity (feFlood/feDropShadow, 0-1) and values (feColorMatrix type hueRotate or saturate, one number) with scalar; gradient stop elements (targetId = the stop id) offset (0-1, kept monotonic), stop-opacity and stop-color (rgba). interpolation: step, linear or cubic. trackId: 1-128 ASCII letters, digits, `_` or `-`. Optional keys: [{tick, value, interp?}] (at most 256; value forms and interp as in native_timeline_set_property_key) are applied in the same undoable edit. Optional noise {amplitude, frequency?(Hz, default 4), seed?, octaves?(1-4)} adds deterministic seeded smooth noise to a scalar, transform2d (e,f) or trs (tx,ty,rotation) track for shake, flicker, sway or hand-drawn boil; it is a pure function of tick and seed, so frames and exports repeat exactly. Example: {\"trackId\":\"fade\",\"targetId\":\"vexel-object-1\",\"property\":\"opacity\",\"valueType\":\"scalar\",\"interpolation\":\"linear\",\"value\":0,\"keys\":[{\"tick\":12,\"value\":1}]}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"targetId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"property\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"valueType\":{\"type\":\"string\",\"enum\":[\"scalar\",\"rgba\",\"transform2d\",\"trs\",\"boolean\",\"path\"]},\"interpolation\":{\"type\":\"string\",\"enum\":[\"step\",\"linear\",\"cubic\"]},\"value\":{\"type\":[\"number\",\"boolean\",\"array\",\"object\",\"string\"]},\"name\":{\"type\":\"string\",\"maxLength\":256},\"noise\":{\"type\":\"object\",\"properties\":{\"amplitude\":{\"type\":\"number\",\"minimum\":0,\"maximum\":100000},\"frequency\":{\"type\":\"number\",\"exclusiveMinimum\":0,\"maximum\":240},\"seed\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"octaves\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":4}},\"required\":[\"amplitude\"],\"additionalProperties\":false},\"keys\":{\"type\":\"array\",\"maxItems\":256,\"items\":{\"type\":\"object\",\"properties\":{\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"value\":{\"type\":[\"number\",\"boolean\",\"array\",\"object\",\"string\"]},\"ease\":{\"type\":[\"string\",\"array\"]},\"interp\":{\"type\":\"string\"}},\"required\":[\"tick\",\"value\"],\"additionalProperties\":false}}}",
        required: "documentId,trackId,targetId,property,valueType,interpolation,value",
        fields: "documentId:string;trackId:string;targetId:string;property:string;valueType:string;interpolation:string;value:number|boolean|array|object|string;name:string;noise:object;keys:array",
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
        description: "Set or replace one undoable key on a property track (not a drawing track). tick: integer 0 through durationTicks-1 (0-47 on the default 48-tick timeline; lengthen it with native_timeline_set_duration). value must match the track valueType: a number for scalar, true/false for boolean, an array of 4 numbers for rgba, 6 for transform2d, a path data string for path (same command structure as the track), and for trs either an array [tx,ty,rotationDeg,sx,sy,pivotX,pivotY(,skewXDeg)] or an object with optional translate ([x,y]; or tx/ty), rotate (degrees), scale (number or [x,y]; or sx/sy), pivot ([x,y]; or pivotX/pivotY) and skewX (degrees), omitted parts being identity; optional ease (shapes the segment from this key to the next; works for every value type) is step, linear, ease_in, ease_out, ease_in_out (quad), ease_in_cubic, ease_out_cubic, ease_in_out_cubic, ease_in_back, ease_out_back, ease_in_out_back, ease_in_elastic, ease_out_elastic, ease_in_out_elastic, ease_in_bounce, ease_out_bounce, ease_in_out_bounce (aliases back, elastic, bounce = the out forms), a string bezier:x1,y1,x2,y2 or an array [x1,y1,x2,y2] (x in 0..1, y in -4..4). Examples: {\"trackId\":\"fade\",\"tick\":12,\"value\":1}; {\"trackId\":\"swing\",\"tick\":12,\"value\":{\"rotate\":20,\"pivot\":[250,400]}}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"value\":{\"type\":[\"number\",\"boolean\",\"array\",\"object\",\"string\"]},\"valueType\":{\"type\":\"string\",\"enum\":[\"scalar\",\"rgba\",\"transform2d\",\"trs\",\"boolean\",\"path\"]},\"ease\":{\"type\":[\"string\",\"array\"]}}",
        required: "documentId,trackId,tick,value",
        fields: "documentId:string;trackId:string;tick:integer;value:number|boolean|array|object|string;valueType:string;ease:string|array",
    },
    ToolSchema {
        name: "native_timeline_set_cel",
        description: "Show asset assetName on a drawing track from tick until the next cel; a tick that already has a cel is replaced. tick: integer 0 through durationTicks-1 (0-47 on the default 48-tick timeline; lengthen it with native_timeline_set_duration). The asset must exist (native_timeline_add_drawing_asset) and trackId must be a drawing track (native_timeline_add_track, which already has a cel at tick 0). Example: {\"trackId\":\"ball\",\"tick\":8,\"assetName\":\"ball-low\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"assetName\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"label\":{\"type\":\"string\",\"maxLength\":256}}",
        required: "documentId,trackId,tick,assetName",
        fields: "documentId:string;trackId:string;tick:integer;assetName:string;label:string",
    },
    ToolSchema {
        name: "native_timeline_remove_cel",
        description: "Remove the cel at tick from a drawing track. The tick must hold a cel (see native_timeline_get) and the last remaining cel of a track cannot be removed.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"tick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}}",
        required: "documentId,trackId,tick",
        fields: "documentId:string;trackId:string;tick:integer",
    },
    ToolSchema {
        name: "native_timeline_move_item",
        description: "Move the cel or property key at fromTick to the unused integer toTick on trackId. Ticks are 0-47 on the default 48-tick timeline; the item at tick 0 cannot be moved. Example: {\"trackId\":\"ball\",\"fromTick\":8,\"toTick\":10}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"fromTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"toTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}}",
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
        fields: "documentId:string;path:string;format:string;fps:integer;duration:number;codec:string;quality:number;bitrate:integer;width:integer;height:integer;loops:integer;background:string;alpha:boolean",
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

/// Tools the live preview advertises and routes. The `tools/list` filter and
/// the `tools/call` gate both read this list; `vexel-app` dispatches each one.
pub(crate) const PREVIEW_TOOLS: &[&str] = &[
    "document_list",
    "document_info",
    "element_add",
    "element_add_fragment",
    "use_add",
    "undo",
    "redo",
    "animation_list",
    "animation_add",
    "animation_control",
    "export_animation",
    "document_open",
    "attribute_set",
    "element_delete",
    "save",
    "save_as",
    "export_svg",
    "render_png",
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
    "element_group",
    "element_ungroup",
    "element_duplicate",
    "element_reorder",
    "element_reparent",
    "element_get",
    "element_query",
    "tree_get",
    "attribute_remove",
    "text_set",
    "gradient_add",
    "gradient_set_stop",
    "element_set_paint",
    "clip_add",
    "element_set_clip",
    "element_set_transform",
    "text_add",
    "native_timeline_remove_key",
    "native_timeline_move_key",
    "native_timeline_remove_track",
    "native_timeline_rename_track",
    "native_timeline_duplicate_track",
    "native_timeline_set_interpolation",
    "native_timeline_reorder_track",
    "native_timeline_set_duration",
    "batch",
    "native_timeline_tracks_set",
    "native_timeline_set_camera",
    "audio_add_track",
    "audio_set",
    "audio_list",
    "audio_generate",
    "audio_envelope",
    "rig_create",
    "rig_pose",
    "clip_apply",
    "mouth_set",
    "lip_sync",
    "eyes_set",
    "blink",
    "image_trace",
    "image_segment",
    "image_inpaint",
    "document_import",
    "scene_apply",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolValidationError {
    UnknownTool,
    UnexpectedArgument,
    /// The named argument has a type the tool does not accept.
    WrongType(&'static str),
    MissingRequired,
    InvalidLoopMode,
    InvalidTemplateAttributes,
    InvalidTransactionLength,
    /// A batch holds fewer than 1 or more than 256 operations.
    InvalidBatchLength,
    /// A batch operation names a tool that cannot run inside a batch.
    BatchToolNotAllowed,
    InvalidArguments,
    /// One batch operation failed validation; the message names its index,
    /// tool and offending argument.
    BatchOp(String),
}

/// Preview-only tools: paint/transform/text and native timeline editing.
fn preview_extra_tools() -> impl Iterator<Item = &'static ToolSchema> {
    super::tools_paint::PAINT_TOOLS
        .iter()
        .chain(super::tools_native_edit::NATIVE_EDIT_TOOLS)
        .chain(super::tools_native_bulk::NATIVE_BULK_TOOLS)
        .chain(super::tools_audio::AUDIO_TOOLS)
        .chain(super::tools_structure::STRUCTURE_TOOLS)
        .chain(super::tools_rig::RIG_TOOLS)
        .chain(super::tools_face::FACE_TOOLS)
        .chain(super::tools_batch::BATCH_TOOLS)
        .chain(super::tools_image::IMAGE_TOOLS)
        .chain(super::tools_scene::SCENE_TOOLS)
}

pub fn tool_schemas() -> Result<Vec<u8>, ProtocolError> {
    tool_schemas_for_preview(false)
}

pub(super) fn tool_schemas_for_preview(preview: bool) -> Result<Vec<u8>, ProtocolError> {
    let mut out = JsonBuffer::new(4096, MAX_BYTES).map_err(ProtocolError::Buffer)?;
    out.literal("[").map_err(ProtocolError::Buffer)?;
    let selected: Vec<_> = TOOLS
        .iter()
        .chain(preview_extra_tools().filter(|_| preview))
        .filter(|tool| {
            !(tool.name == "animation_control" && !preview)
                && (!preview || PREVIEW_TOOLS.contains(&tool.name))
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
            element_add_description()
        } else if preview && tool.name == "export_animation" {
            "Export the live preview loop inside a granted directory. gif (fps 1-30, duration 0.01-10 seconds) works for any animation. mp4 and mov (H.264 or HEVC) need a native timeline and export its whole loop; fps defaults to the timeline tick rate, extension must match format, width and height go up to 4096 and must be given together, quality (0-1, default 0.5; constant-quality mode, encoder level 0.6 + 0.4*quality, so size also scales with resolution, fps and content) and bitrate (kbps) are alternatives, loops repeats the loop, background (#rrggbb) fills transparency, alpha true keeps transparency (mov with hevc only; background is ignored). apng writes an animated PNG (path ends .png or .apng) of one loop, keeping alpha unless background is given; loops is the play count (default forever). frames writes one PNG per sampled tick into the path's directory: path is name_####.png (4+ #) or a name prefix without extension (name_0000.png...); it refuses to overwrite and caps at 1440 frames. apng and frames take fps, width and height (together) and background, and need a native timeline.".to_owned()
        } else if preview && tool.name == "render_png" {
            "Render the open document to a PNG file inside a granted directory. Optional time is in SECONDS (tick / ticksPerSecond; 0 to the timeline length, default 0), not ticks; a time past the end of the timeline is rejected.".to_owned()
        } else {
            tool.description.to_owned()
        };
        append_string(&mut out, description.as_bytes())?;
        out.literal(",\"inputSchema\":{\"type\":\"object\",\"properties\":")
            .map_err(ProtocolError::Buffer)?;
        if preview && tool.name == "element_add" {
            out.literal(r#"{"documentId":{"type":"string"},"parentId":{"type":"string"},"template":{"type":"string","enum":["rect","ellipse","line","path"]},"attributes":{"type":"object","additionalProperties":{"type":"string"}},"id":{"type":"string","minLength":1,"maxLength":128},"opacity":{"type":"number","minimum":0,"maximum":1},"transform":{"type":"object","properties":{"translateX":{"type":"number"},"translateY":{"type":"number"},"rotate":{"type":"number"},"pivotX":{"type":"number"},"pivotY":{"type":"number"},"scaleX":{"type":"number"},"scaleY":{"type":"number"},"skewX":{"type":"number"}},"additionalProperties":false},"gradientId":{"type":"string","minLength":1,"maxLength":128},"paint":{"type":"string","enum":["fill","stroke"]}}"#).map_err(ProtocolError::Buffer)?;
        } else {
            let properties = if preview && tool.name == "animation_add" {
                r#"{"documentId":{"type":"string"},"targetId":{"type":"string","minLength":1,"maxLength":128},"xml":{"type":"string","maxLength":4096,"pattern":"^<animate\\s[^>]*\\s/>$"}}"#
            } else if preview && tool.name == "animation_control" {
                r#"{"documentId":{"type":"string"},"action":{"type":"string","enum":["play","pause","seek"]},"timeMs":{"type":"number","minimum":0,"maximum":10000}}"#
            } else if preview && tool.name == "export_animation" {
                r#"{"documentId":{"type":"string"},"path":{"type":"string","minLength":1,"maxLength":1023},"format":{"type":"string","enum":["gif","mp4","mov","apng","frames"]},"fps":{"type":"integer","minimum":1,"maximum":120},"duration":{"type":"number","minimum":0.01,"maximum":10},"codec":{"type":"string","enum":["h264","hevc"]},"quality":{"type":"number","minimum":0,"maximum":1},"bitrate":{"type":"integer","minimum":50,"maximum":200000},"width":{"type":"integer","minimum":1,"maximum":4096},"height":{"type":"integer","minimum":1,"maximum":4096},"loops":{"type":"integer","minimum":1,"maximum":100},"background":{"type":"string","pattern":"^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$"},"alpha":{"type":"boolean"}}"#
            } else if preview && tool.name == "render_png" {
                r#"{"documentId":{"type":"string"},"path":{"type":"string","minLength":1,"maxLength":1023},"width":{"type":"integer","minimum":1,"maximum":4096},"height":{"type":"integer","minimum":1,"maximum":4096},"time":{"type":"number","minimum":0,"maximum":3600}}"#
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
        } else if preview && tool.name == "export_animation" {
            "documentId,path,format"
        } else if preview && tool.name == "render_png" {
            "documentId,path"
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
    validate_arguments(doc, arguments, tool_name, false, &mut None)
}

/// `inherit_document` treats `documentId` as supplied (a batch operation takes
/// the batch's). `field` receives the offending or missing argument name.
pub(super) fn validate_arguments(
    doc: &JsonDocument<'_>,
    arguments: usize,
    tool_name: &str,
    inherit_document: bool,
    field_out: &mut Option<String>,
) -> Result<(), ToolValidationError> {
    let Some(args) = doc.token(arguments) else {
        return Err(ToolValidationError::InvalidArguments);
    };
    if args.kind != JsonType::Object {
        return Err(ToolValidationError::InvalidArguments);
    }
    let Some(tool) = TOOLS
        .iter()
        .chain(preview_extra_tools())
        .find(|t| t.name == tool_name)
    else {
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
        *field_out = Some(String::from_utf8_lossy(&key_bytes).into_owned());
        let Ok(field) = std::str::from_utf8(&key_bytes) else {
            return Err(ToolValidationError::UnexpectedArgument);
        };
        let Some((field_name, expected)) = tool.fields.split(';').find_map(|field_spec| {
            let (name, kind) = field_spec.split_once(':')?;
            (name == field).then_some((name, kind))
        }) else {
            return Err(ToolValidationError::UnexpectedArgument);
        };
        if !expected
            .split('|')
            .any(|kind| type_matches(doc, value, kind))
        {
            return Err(ToolValidationError::WrongType(field_name));
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
        if tool.name == "batch" && field == "ops" {
            super::tools_batch::validate_ops(doc, arguments, value)?;
        }
        if tool.name == "native_timeline_tracks_set" && field == "tracks" {
            let count = doc.token(value).map_or(0, |t| t.child_count);
            if !(1..=super::tools_native_bulk::MAX_TRACKS).contains(&count) {
                return Err(ToolValidationError::InvalidArguments);
            }
        }
        if tool.name == "scene_apply" && matches!(field, "elements" | "tracks") {
            let count = doc.token(value).map_or(0, |t| t.child_count);
            let max = if field == "tracks" {
                super::tools_native_bulk::MAX_TRACKS
            } else {
                super::tools_scene::MAX_ELEMENTS
            };
            if !(1..=max).contains(&count) {
                return Err(ToolValidationError::InvalidArguments);
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
    // mp4, mov, apng and frames take fps optionally and no duration.
    let video_export = tool.name == "export_animation"
        && doc.object_get(arguments, b"format").is_some_and(|v| {
            [&b"mp4"[..], b"mov", b"apng", b"frames"]
                .iter()
                .any(|f| doc.string_equals(v, f))
        });
    for required in tool.required.split(',').filter(|s| !s.is_empty()) {
        if video_export && matches!(required, "fps" | "duration") {
            continue;
        }
        if inherit_document && required == "documentId" {
            continue;
        }
        if doc.object_get(arguments, required.as_bytes()).is_none() {
            *field_out = Some(required.to_owned());
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
