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
    ToolSchema {
        name: "audio_generate",
        description: "Generate a WAV (PCM16, 48 kHz mono) with the optional local audio helper and, with addTrack, add it as an audio track in the same undoable edit. kind voice: params {text (1-1000 chars), voice (Kokoro id, default af_heart; also af_sky, am_onyx, ...), speed 0.5-2, effect pip|whisper|soft, or pitchSemitones, bitcrushBits, tremoloHz, tremoloDepth, ringModHz, ringModMix, highpassHz, lowpassHz}; needs the Kokoro model: without it the call fails with model_missing and the exact install steps, and the model is downloaded only when consent is true (ask the user first). kind sfx: params {preset whoosh|hum|launch|chime|siren|wind|comet|hoverbike, durationMs 200-15000 (whoosh, hum, siren, wind), seed}. kind music: params {mood adventure|calm|tense|bright, key like A or C#, tempo 50-220, bars 2-64, seed}; the loop is seamless. sfx and music are deterministic for the same params and need no model. outPath is a .wav file in a granted directory (inside the document's directory with addTrack, which needs a saved document); addTrack {trackId, startTick, gainDb, trimInMs, trimOutMs, fadeInTicks, fadeOutTicks, loop, duck} takes the same values as audio_add_track. Limits: 120 s per voice request, 60 s otherwise, clips up to 120 s and 32 MiB, 128 requests per session. Example: {\"kind\":\"sfx\",\"params\":{\"preset\":\"chime\"},\"outPath\":\"audio/chime.wav\",\"addTrack\":{\"trackId\":\"chime\"}}.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"kind\":{\"type\":\"string\",\"enum\":[\"voice\",\"sfx\",\"music\"]},\"params\":{\"type\":\"object\"},\"outPath\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":1023},\"consent\":{\"type\":\"boolean\"},\"addTrack\":{\"type\":\"object\",\"properties\":{\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"startTick\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"trimInMs\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":600000},\"trimOutMs\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":600000},\"gainDb\":{\"type\":\"number\",\"minimum\":-120,\"maximum\":24},\"fadeInTicks\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"fadeOutTicks\":{\"type\":\"integer\",\"minimum\":0,\"maximum\":4294967295},\"loop\":{\"type\":\"boolean\"},\"duck\":{\"type\":\"object\"}},\"required\":[\"trackId\"]}}",
        required: "documentId,kind,outPath",
        fields: "documentId:string;kind:string;params:object;outPath:string;consent:boolean;addTrack:object",
    },
    ToolSchema {
        name: "audio_envelope",
        description: "Per-tick loudness of an audio file or track at the timeline tick rate, for lip-sync and audio-reactive animation. Pass path (WAV, AIFF or CAF in a granted directory, absolute or relative to the saved document) or trackId (the track's trimmed region, starting at its startTick). Returns {tickRate, startTick, ticks, durationMs, peakRms, rms:[...]} where rms holds one value per tick, scaled so the loudest tick is 1 (peakRms is the unscaled maximum). ticks limits the count (1-100000, default the whole clip). Deterministic and computed locally; needs a native timeline for the tick rate.",
        properties: "{\"documentId\":{\"type\":\"string\"},\"path\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":1023},\"trackId\":{\"type\":\"string\",\"minLength\":1,\"maxLength\":128},\"ticks\":{\"type\":\"integer\",\"minimum\":1,\"maximum\":100000}}",
        required: "documentId",
        fields: "documentId:string;path:string;trackId:string;ticks:integer",
    },
];
