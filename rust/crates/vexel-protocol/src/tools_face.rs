//! Schemas of the face tools: `mouth_set`, `lip_sync`, `eyes_set`, `blink`.
use super::ToolSchema;

pub(crate) const FACE_TOOLS: &[ToolSchema] = &[
    ToolSchema {
    name: "mouth_set",
    description: "Define the mouth shapes of a character: which <g> group shows for each viseme, stored in the document (a <desc id=\"vexel-mouths\"> element) so it saves, reopens and undoes with the file; one undoable edit, calling again for the same character replaces its set. character is a <g> group id; shapes maps viseme names to group ids inside the character. Visemes (at most 12): rest, A, E, I, O, U, MBP, FV, L, WQ, CDGK, TH. rest is required; a missing viseme falls back to the nearest shape given (then rest). Several visemes may share one group. At most 32 characters per document. Renaming a shape group with attribute_set id keeps the set pointing at it. Example: {\"character\":\"hero\",\"shapes\":{\"rest\":\"m_rest\",\"A\":\"m_open\",\"O\":\"m_round\"}}.",
    properties: "{\"documentId\":{\"type\":\"string\"},\"character\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"shapes\":{\"type\":\"object\"}}",
    required: "documentId,character,shapes",
    fields: "documentId:string;character:string;shapes:object",
    },
    ToolSchema {
        name: "lip_sync",
        description: "Animate a character's mouth from audio in ONE undoable edit: derives a viseme per tick from the loudness envelope (see audio_envelope) and writes a step `visibility` track per mouth group of the character's mouth_set (tracks lip_<group>; earlier lip tracks of those groups are replaced). source is an object with exactly one of trackId (an audio track: its trimmed region, starting at its startTick), path (WAV, AIFF or CAF in a granted directory) or envelope (1-100000 loudness numbers 0-1, one per tick). startTick (default the track's start, else 0) places the first tick; ticks limits a trackId/path envelope (1-100000). Closed under mapping.close (default 0.08), then small/medium/wide openings (mapping.mid 0.35, mapping.wide 0.65) pick O/U, E/O/I or A/E with a deterministic variation seeded by seed (default 0) that holds a vowel for 6 ticks, so the mouth does not flap; smoothing (0-8, default 1) is a moving-average radius. A voiced run after silence opens on MBP when the set has it. text is an optional list of up to 256 words spoken in turn, one per voiced run (vowels a e i o u, b/m/p MBP, f/v FV, l L, w/q WQ, th TH, other consonants CDGK). Ticks at or past the timeline duration are dropped and counted in droppedTicks. At most 16384 keys. Hidden parents hide their children as in SVG, so keep the mouth groups under a visible parent. Needs a mouth_set for the character.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"character\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"source\":{\"type\":\"object\"},\"startTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"ticks\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":100000},\"mapping\":{\"type\":\"object\"},\"smoothing\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":8},\"seed\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"text\":{\"type\":\"array\",\"maxItems\":256,\"items\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":32}}}",
        required: "documentId,character,source",
        fields: "documentId:string;character:string;source:object;startTick:integer;ticks:integer;mapping:object;smoothing:integer;seed:integer;text:array",
    },
];
