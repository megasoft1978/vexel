//! Schemas of the gradient, clip, transform and text tools (wired into the
//! tool list by the coordinator).

use super::ToolSchema;

pub(crate) const PAINT_TOOLS: &[ToolSchema] = &[
    ToolSchema {
        name: "gradient_add",
        description: "Add a linear or radial gradient to the document defs and return its gradientId and stopIds. kind is \"linear\" or \"radial\". Linear uses x1,y1,x2,y2 (default 0,0,1,0); radial uses cx,cy,r,fx,fy (default 0.5,0.5,0.5, focal point defaults to the centre); all finite numbers, r > 0. units is \"objectBoundingBox\" (default, coordinates 0-1 of the shape box) or \"userSpace\"; spread is \"pad\" (default), \"reflect\" or \"repeat\". stops is a string \"offset:#rrggbb:opacity,...\" with 2-16 stops, offsets 0-1 ascending, opacity 0-1 (default 1). Example: {\"kind\":\"linear\",\"stops\":\"0:#ff0000:1,1:#0000ff:1\"}. Apply it with element_set_paint.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"kind\":{\"type\":\"string\",\"enum\":[\"linear\",\"radial\"]},\"units\":{\"type\":\"string\",\"enum\":[\"userSpace\",\"objectBoundingBox\"]},\"spread\":{\"type\":\"string\",\"enum\":[\"pad\",\"reflect\",\"repeat\"]},\"x1\":{\"type\":\"number\"},\"y1\":{\"type\":\"number\"},\"x2\":{\"type\":\"number\"},\"y2\":{\"type\":\"number\"},\"cx\":{\"type\":\"number\"},\"cy\":{\"type\":\"number\"},\"r\":{\"type\":\"number\",\"exclusiveMinimum\":0},\"fx\":{\"type\":\"number\"},\"fy\":{\"type\":\"number\"},\"stops\":{\"type\":\"string\",\"maxLength\":512}}",
        required: "documentId,kind,stops",
        fields: "documentId:string;kind:string;units:string;spread:string;x1:number;y1:number;x2:number;y2:number;cx:number;cy:number;r:number;fx:number;fy:number;stops:string",
    },
    ToolSchema {
        name: "gradient_set_stop",
        description: "Edit one stop of a gradient by position. Pass at least one of offset (0-1, must stay ordered between neighbouring stops), color (\"#rrggbb\") or opacity (0-1). index is 0-15. Example: {\"gradientId\":\"vexel-gradient-1\",\"index\":1,\"color\":\"#00ff00\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"gradientId\":{\"type\":\"string\"},\"index\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":15},\"offset\":{\"type\":\"number\",\"minimum\":0,\"maximum\":1},\"color\":{\"type\":\"string\"},\"opacity\":{\"type\":\"number\",\"minimum\":0,\"maximum\":1}}",
        required: "documentId,gradientId,index",
        fields: "documentId:string;gradientId:string;index:integer;offset:number;color:string;opacity:number",
    },
    ToolSchema {
        name: "element_set_paint",
        description: "Paint a rect, ellipse, circle, line or path fill or stroke with a gradient added by gradient_add. property is \"fill\" or \"stroke\". Example: {\"elementId\":\"vexel-object-1\",\"property\":\"fill\",\"gradientId\":\"vexel-gradient-1\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"property\":{\"type\":\"string\",\"enum\":[\"fill\",\"stroke\"]},\"gradientId\":{\"type\":\"string\"}}",
        required: "documentId,elementId,property,gradientId",
        fields: "documentId:string;elementId:string;property:string;gradientId:string",
    },
    ToolSchema {
        name: "clip_add",
        description: "Add a clip path and return its clipId. shape \"rect\" takes a,b,c,d as x,y,width,height; shape \"ellipse\" takes cx,cy,rx,ry. Values are finite numbers up to 1000000; width, height, rx and ry must be > 0. Example: {\"shape\":\"rect\",\"a\":10,\"b\":10,\"c\":100,\"d\":60}. Apply it with element_set_clip.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"shape\":{\"type\":\"string\",\"enum\":[\"rect\",\"ellipse\"]},\"a\":{\"type\":\"number\"},\"b\":{\"type\":\"number\"},\"c\":{\"type\":\"number\"},\"d\":{\"type\":\"number\"}}",
        required: "documentId,shape,a,b,c,d",
        fields: "documentId:string;shape:string;a:number;b:number;c:number;d:number",
    },
    ToolSchema {
        name: "element_set_clip",
        description: "Clip a shape or group with a clip path from clip_add. Omit clipId to remove the clip. Example: {\"elementId\":\"vexel-object-1\",\"clipId\":\"vexel-clip-1\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"clipId\":{\"type\":\"string\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string;clipId:string",
    },
    ToolSchema {
        name: "element_set_transform",
        description: "Replace an element's transform. Applied as translate, then rotate (degrees, about pivotX,pivotY), scale and skewX (degrees). Missing values are identity (translate 0, rotate 0, pivot 0,0, scale 1, skew 0); all identity removes the transform. All finite numbers. Example: {\"elementId\":\"vexel-object-1\",\"translateX\":20,\"rotate\":45,\"pivotX\":50,\"pivotY\":50}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"elementId\":{\"type\":\"string\"},\"translateX\":{\"type\":\"number\"},\"translateY\":{\"type\":\"number\"},\"rotate\":{\"type\":\"number\"},\"pivotX\":{\"type\":\"number\"},\"pivotY\":{\"type\":\"number\"},\"scaleX\":{\"type\":\"number\"},\"scaleY\":{\"type\":\"number\"},\"skewX\":{\"type\":\"number\"}}",
        required: "documentId,elementId",
        fields: "documentId:string;elementId:string;translateX:number;translateY:number;rotate:number;pivotX:number;pivotY:number;scaleX:number;scaleY:number;skewX:number",
    },
    ToolSchema {
        name: "text_add",
        description: "Add a text element at baseline position x,y (finite numbers up to 1000000) and return its elementId. content is 1-256 characters without control characters; fontSize 1-1024 (default 16); anchor \"start\" (default), \"middle\" or \"end\"; fill \"#rrggbb\" (default #000000). Example: {\"x\":20,\"y\":40,\"content\":\"Hello\",\"fontSize\":24,\"fill\":\"#336699\"}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"x\":{\"type\":\"number\"},\"y\":{\"type\":\"number\"},\"content\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":256},\"fontSize\":{\"type\":\"number\",\"minimum\":1,\"maximum\":1024},\"anchor\":{\"type\":\"string\",\"enum\":[\"start\",\"middle\",\"end\"]},\"fill\":{\"type\":\"string\"}}",
        required: "documentId,x,y,content",
        fields: "documentId:string;x:number;y:number;content:string;fontSize:number;anchor:string;fill:string",
    },
];
