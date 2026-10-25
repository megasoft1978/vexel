//! Schemas of the timeline audio tools.
use super::ToolSchema;

pub(crate) const AUDIO_TOOLS: &[ToolSchema] = &[
    ToolSchema {
        name: "audio_add_track",
        description: "Add an undoable audio track (voice, sfx or music) to the native timeline; export_animation mp4/mov then muxes the mixdown as an AAC track. Save the document first (save_as): path is a WAV, AIFF or CAF file (PCM16/24/32 or float32, mono or stereo, 8-192 kHz, at most 10 minutes and 256 MiB) inside the document's directory, given relative to it or absolute; it is stored relative and re-checked against the access grants on every use. startTick places the clip on the timeline (below durationTicks), trimInMs/trimOutMs select part of the file, gainDb is -120 to 24, fadeInTicks/fadeOutTicks are linear fades, loop repeats the trimmed region until the timeline ends, duck {under, amountDb} lowers this track by amountDb (-60 to 0) whenever the named track id, or any track of kind voice/sfx/music, is audible. At most 64 tracks. The mix is 48 kHz stereo with a peak limiter; loops of the video do not repeat the audio. Example: {\"trackId\":\"narration\",\"kind\":\"voice\",\"path\":\"audio/line1.wav\",\"startTick\":12,\"gainDb\":-3}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"kind\":{\"type\":\"string\",\"enum\":[\"voice\",\"sfx\",\"music\"]},\"path\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":1023},\"startTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"trimInMs\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":600000},\"trimOutMs\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":600000},\"gainDb\":{\"type\":\"number\",\"minimum\":-120,\"maximum\":24},\"fadeInTicks\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"fadeOutTicks\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"loop\":{\"type\":\"boolean\"},\"duck\":{\"type\":\"object\",\"properties\":{\"under\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"amountDb\":{\"type\":\"number\",\"minimum\":-60,\"maximum\":0}},\"required\":[\"under\",\"amountDb\"],\"additionalProperties\":false}}",
        required: "documentId,trackId,kind,path",
        fields: "documentId:string;trackId:string;kind:string;path:string;startTick:integer;trimInMs:integer;trimOutMs:integer;gainDb:number;fadeInTicks:integer;fadeOutTicks:integer;loop:boolean;duck:object",
    },
    ToolSchema {
        name: "audio_set",
        description: "Change or remove one audio track in an undoable edit. Pass only what changes: gainDb, fadeInTicks, fadeOutTicks, startTick (move), trimInMs, trimOutMs (0 clears the trim end), loop, duck {under, amountDb} or clearDuck:true; remove:true deletes the track (not while another track ducks under it). Ids and ranges as in audio_add_track.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"remove\":{\"type\":\"boolean\"},\"startTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"trimInMs\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":600000},\"trimOutMs\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":600000},\"gainDb\":{\"type\":\"number\",\"minimum\":-120,\"maximum\":24},\"fadeInTicks\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"fadeOutTicks\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"loop\":{\"type\":\"boolean\"},\"duck\":{\"type\":\"object\",\"properties\":{\"under\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"amountDb\":{\"type\":\"number\",\"minimum\":-60,\"maximum\":0}},\"required\":[\"under\",\"amountDb\"],\"additionalProperties\":false},\"clearDuck\":{\"type\":\"boolean\"}}",
        required: "documentId,trackId",
        fields: "documentId:string;trackId:string;remove:boolean;gainDb:number;fadeInTicks:integer;fadeOutTicks:integer;startTick:integer;trimInMs:integer;trimOutMs:integer;loop:boolean;duck:object;clearDuck:boolean",
    },
    ToolSchema {
        name: "audio_list",
        description: "List the timeline's audio tracks: trackId, kind, stored relative path, startTick, trim, gainDb, fades, loop and duck.",
        properties: "{\"documentId\":{\"type\":\"string\"}}",
        required: "documentId",
        fields: "documentId:string",
    },
];
