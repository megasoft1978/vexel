//! Schemas of the preview-only `element_reparent` and `use_add` tools.
use super::ToolSchema;

pub(crate) const STRUCTURE_TOOLS: &[ToolSchema] = &[
    ToolSchema {
    name: "element_reparent",
    description: "Move an element into another group (or back to the root) in one undoable edit. parentId is \"root\" or a group id that is neither the element nor inside it. index is the zero-based position among the destination's other children; omitted or past the end appends. The element keeps its id; transforms of the old and new ancestors are not compensated. Example: {\"elementId\":\"vexel-object-3\",\"parentId\":\"vexel-object-9\",\"index\":0}.",
    properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"parentId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"index\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":100000}}",
    required: "documentId,elementId,parentId",
    fields: "documentId:string;elementId:string;parentId:string;index:integer",
},
    ToolSchema {
        name: "use_add",
        description: "Instantiate an existing element, group or symbol by reference: adds <use href=\"#refId\"> as the last child of parentId (default \"root\") and returns the new elementId. The instance counts as ONE element toward the scene limit and is animatable (transform and opacity tracks target the use id), selectable, movable, undoable and saved. x/y translate the instance; transform is the element_add transform object (applied before x/y); fill, stroke (#rrggbb, none, currentColor) and opacity (0-1) are inherited by referenced content that sets none of its own. refId must not contain parentId. Put reusable artwork in <defs> with element_add_fragment first. Example: {\"refId\":\"star\",\"x\":40,\"y\":30,\"id\":\"star-1\",\"fill\":\"#00ff00\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"parentId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"refId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"id\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"x\":{\"type\":\"number\"},\"y\":{\"type\":\"number\"},\"transform\":{\"type\":\"object\",\"properties\":{\"translateX\":{\"type\":\"number\"},\"translateY\":{\"type\":\"number\"},\"rotate\":{\"type\":\"number\"},\"pivotX\":{\"type\":\"number\"},\"pivotY\":{\"type\":\"number\"},\"scaleX\":{\"type\":\"number\"},\"scaleY\":{\"type\":\"number\"},\"skewX\":{\"type\":\"number\"}},\"additionalProperties\":false},\"fill\":{\"type\":\"string\"},\"stroke\":{\"type\":\"string\"},\"opacity\":{\"type\":\"number\",\"minimum\":0,\"maximum\":1}}",
        required: "documentId,refId",
        fields: "documentId:string;parentId:string;refId:string;id:string;x:number;y:number;transform:object;fill:string;stroke:string;opacity:number",
    },
];
