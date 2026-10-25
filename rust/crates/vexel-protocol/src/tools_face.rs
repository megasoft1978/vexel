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
    ToolSchema {
        name: "eyes_set",
        description: "Keep pupils inside their eyes in ONE undoable edit. eyes: 1-8 of {eye, pupils}: eye is the id of the eye-shape element (ellipse, circle, rect, path or group; its bounds define the outline: a rect clips to a rectangle, anything else to the inscribed ellipse), pupils are 1-8 sibling <g> groups (pupil, iris, highlight). The pupils are wrapped in a new group that carries a generated clipPath, so any later pupil motion (blink gaze keys, rig poses, transforms) stays inside the eye. Neither the eye nor the pupils' parent may sit inside a transformed group. Returns each eye's wrapper groupId and clipId. Example: {\"eyes\":[{\"eye\":\"eye_l\",\"pupils\":[\"pupil_l\",\"shine_l\"]}]}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"eyes\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":8,\"items\":{\"type\":\"object\",\"properties\":{\"eye\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"pupils\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":8,\"items\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128}}},\"required\":[\"eye\",\"pupils\"],\"additionalProperties\":false}}}",
        required: "documentId,eyes",
        fields: "documentId:string;eyes:array",
    },
    ToolSchema {
        name: "blink",
        description: "Generate blink and gaze animation in ONE undoable edit (tracks are replaced when a blink runs again). Give lids, swap and/or gaze. lids: 1-16 of {id, pivot?, closedScale?}: a <g> group squashed vertically (scale-y to closedScale, default 0.1) about pivot ([x,y] or \"center\", default the group's bounds centre) and back, as a trs track blink_<id>. swap: {open: [ids shown between blinks], closed: [ids shown during a blink]} as step visibility tracks. gaze: {pupils: [ids], keys: [{tick, x, y}]} translates each pupil group to (x,y) at each tick (linear, tracks gaze_<id>; put the pupils under eyes_set so they stay in the eye). When (required with lids or swap): exactly one of at (1-256 blink start ticks, each at least duration after the previous) or random {seed?, start?, end?, minGap?, maxGap?} (deterministic from seed: gaps of minGap..maxGap ticks, default 24..96, between start and end, default the timeline duration). duration is the ticks a blink lasts (2-48, default 4): the eye is closed at the half. All ticks must be below the timeline duration.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"lids\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":16},\"swap\":{\"type\":\"object\"},\"gaze\":{\"type\":\"object\"},\"at\":{\"type\":\"array\",\"minItems\":1,\"maxItems\":256,\"items\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295}},\"random\":{\"type\":\"object\"},\"duration\":{\"type\":\"integer\",\"minimum\":2,\"maximum\":48}}",
        required: "documentId",
        fields: "documentId:string;lids:array;swap:object;gaze:object;at:array;random:object;duration:integer",
    },
];
