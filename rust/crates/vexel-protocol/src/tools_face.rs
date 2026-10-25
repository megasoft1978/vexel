//! Schemas of the face tools: `mouth_set`, `lip_sync`, `eyes_set`, `blink`.
use super::ToolSchema;

pub(crate) const FACE_TOOLS: &[ToolSchema] = &[ToolSchema {
    name: "mouth_set",
    description: "Define the mouth shapes of a character: which <g> group shows for each viseme, stored in the document (a <desc id=\"vexel-mouths\"> element) so it saves, reopens and undoes with the file; one undoable edit, calling again for the same character replaces its set. character is a <g> group id; shapes maps viseme names to group ids inside the character. Visemes (at most 12): rest, A, E, I, O, U, MBP, FV, L, WQ, CDGK, TH. rest is required; a missing viseme falls back to the nearest shape given (then rest). Several visemes may share one group. At most 32 characters per document. Renaming a shape group with attribute_set id keeps the set pointing at it. Example: {\"character\":\"hero\",\"shapes\":{\"rest\":\"m_rest\",\"A\":\"m_open\",\"O\":\"m_round\"}}.",
    properties: "{\"documentId\":{\"type\":\"string\"},\"character\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"shapes\":{\"type\":\"object\"}}",
    required: "documentId,character,shapes",
    fields: "documentId:string;character:string;shapes:object",
}];
