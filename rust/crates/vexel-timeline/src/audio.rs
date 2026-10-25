//! Audio tracks of the native timeline: model, validation and the optional
//! root `audio` array of the `.vexel` timeline JSON.
//!
//! Source files are stored by relative path (inside the granted directory the
//! caller resolves them against); this module never touches the filesystem.
use crate::json::{array_spans, object_fields, Field};
use crate::schema::{boolean, field, number, string, uint};
use crate::{DecodeFailure, Error, Result};
use vexel_base::Span;

/// Most audio tracks one timeline may carry.
pub const MAX_AUDIO_TRACKS: usize = 64;
/// Longest stored source path, in bytes.
pub const MAX_AUDIO_PATH_BYTES: usize = 512;
/// Longest trim offset, in milliseconds (10 minutes).
pub const MAX_TRIM_MS: u32 = 600_000;
pub const MIN_GAIN_DB: f64 = -120.0;
pub const MAX_GAIN_DB: f64 = 24.0;
pub const MIN_DUCK_DB: f64 = -60.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioKind {
    Voice,
    Sfx,
    Music,
}
impl AudioKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Voice => "voice",
            Self::Sfx => "sfx",
            Self::Music => "music",
        }
    }
    pub fn parse(text: &[u8]) -> Option<Self> {
        match text {
            b"voice" => Some(Self::Voice),
            b"sfx" => Some(Self::Sfx),
            b"music" => Some(Self::Music),
            _ => None,
        }
    }
}

/// Lowers this track while `under` (a track id or a kind name) is audible.
#[derive(Debug, Clone, PartialEq)]
pub struct Duck {
    pub under: String,
    /// Gain applied while ducked, in dB (`-60..=0`).
    pub amount_db: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioTrack {
    pub id: String,
    pub kind: AudioKind,
    /// Relative path of the source file inside the grant directory.
    pub src: String,
    pub start_tick: u32,
    pub trim_in_ms: u32,
    /// End of the used region in the source; `None` plays to the file end.
    pub trim_out_ms: Option<u32>,
    pub gain_db: f64,
    pub fade_in_ticks: u32,
    pub fade_out_ticks: u32,
    pub looped: bool,
    pub duck: Option<Duck>,
}

fn at(error: Error, offset: usize) -> DecodeFailure {
    DecodeFailure { error, offset, canceled: false }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
        && AudioKind::parse(id.as_bytes()).is_none()
}

/// A relative path with no `..`, empty, absolute, NUL or backslash parts.
pub fn valid_relative_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= MAX_AUDIO_PATH_BYTES
        && !path.starts_with('/')
        && !path.contains(['\0', '\\'])
        && path.split('/').all(|part| !part.is_empty() && part != "." && part != "..")
}

/// Checks every rule that does not need the audio files themselves.
pub fn validate_tracks(tracks: &[AudioTrack], duration_ticks: u32) -> Result<()> {
    if tracks.len() > MAX_AUDIO_TRACKS {
        return Err(at(Error::Limit, 0));
    }
    for (index, track) in tracks.iter().enumerate() {
        if !valid_id(&track.id) {
            return Err(at(Error::InvalidSchema, 0));
        }
        if tracks[..index].iter().any(|prior| prior.id == track.id) {
            return Err(at(Error::Duplicate, 0));
        }
        if !valid_relative_path(&track.src) {
            return Err(at(Error::InvalidValue, 0));
        }
        let trim_ok = track.trim_in_ms <= MAX_TRIM_MS
            && track
                .trim_out_ms
                .is_none_or(|out| out <= MAX_TRIM_MS && out > track.trim_in_ms);
        let fades_ok = track.fade_in_ticks <= duration_ticks && track.fade_out_ticks <= duration_ticks;
        let gain_ok = track.gain_db.is_finite() && (MIN_GAIN_DB..=MAX_GAIN_DB).contains(&track.gain_db);
        if track.start_tick >= duration_ticks || !trim_ok || !fades_ok || !gain_ok {
            return Err(at(Error::InvalidValue, 0));
        }
        if let Some(duck) = &track.duck {
            let by_kind = AudioKind::parse(duck.under.as_bytes()).is_some();
            let by_id = duck.under != track.id && tracks.iter().any(|t| t.id == duck.under);
            if !by_kind && !by_id {
                return Err(at(Error::InvalidReference, 0));
            }
            if !duck.amount_db.is_finite() || !(MIN_DUCK_DB..=0.0).contains(&duck.amount_db) {
                return Err(at(Error::InvalidValue, 0));
            }
        }
    }
    Ok(())
}

fn text(source: &[u8], span: Span) -> Result<String> {
    String::from_utf8(string(source, span)?).map_err(|_| at(Error::InvalidValue, span.start))
}

const REQUIRED: [&[u8]; 9] = [
    b"id",
    b"kind",
    b"src",
    b"start_tick",
    b"trim_in_ms",
    b"gain_db",
    b"fade_in_ticks",
    b"fade_out_ticks",
    b"loop",
];

fn parse_track(source: &[u8], span: Span) -> Result<AudioTrack> {
    let fields = object_fields(source, span, None)?;
    let known = |key: &[u8]| REQUIRED.contains(&key) || matches!(key, b"trim_out_ms" | b"duck");
    if fields.iter().any(|f| !known(&f.key))
        || REQUIRED.iter().any(|key| field(&fields, key).is_none())
    {
        return Err(at(Error::InvalidSchema, span.start));
    }
    let get = |key: &[u8]| field(&fields, key).map(|f| f.value).ok_or_else(|| at(Error::InvalidSchema, span.start));
    let kind_text = string(source, get(b"kind")?)?;
    let kind = AudioKind::parse(&kind_text).ok_or_else(|| at(Error::InvalidValue, span.start))?;
    let trim_out_ms = field(&fields, b"trim_out_ms").map(|f| uint(source, f.value)).transpose()?;
    let duck = match field(&fields, b"duck") {
        None => None,
        Some(f) => {
            let d: Vec<Field> = object_fields(source, f.value, None)?;
            if d.len() != 2 {
                return Err(at(Error::InvalidSchema, f.value.start));
            }
            let under = field(&d, b"under").ok_or_else(|| at(Error::InvalidSchema, f.value.start))?;
            let amount = field(&d, b"amount_db").ok_or_else(|| at(Error::InvalidSchema, f.value.start))?;
            Some(Duck { under: text(source, under.value)?, amount_db: number(source, amount.value)? })
        }
    };
    Ok(AudioTrack {
        id: text(source, get(b"id")?)?,
        kind,
        src: text(source, get(b"src")?)?,
        start_tick: uint(source, get(b"start_tick")?)?,
        trim_in_ms: uint(source, get(b"trim_in_ms")?)?,
        trim_out_ms,
        gain_db: number(source, get(b"gain_db")?)?,
        fade_in_ticks: uint(source, get(b"fade_in_ticks")?)?,
        fade_out_ticks: uint(source, get(b"fade_out_ticks")?)?,
        looped: boolean(source, get(b"loop")?)?,
        duck,
    })
}

/// Parses and validates the value of the root `audio` field.
pub(crate) fn parse_tracks(source: &[u8], span: Span, duration_ticks: u32) -> Result<Vec<AudioTrack>> {
    let spans = array_spans(source, span, MAX_AUDIO_TRACKS, None)?;
    let mut tracks = Vec::new();
    tracks.try_reserve_exact(spans.len()).map_err(|_| at(Error::Limit, span.start))?;
    for item in spans {
        tracks.push(parse_track(source, item)?);
    }
    validate_tracks(&tracks, duration_ticks)?;
    Ok(tracks)
}

fn push_string(out: &mut Vec<u8>, value: &str) {
    out.push(b'"');
    for ch in value.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            c if (c as u32) < 0x20 => out.extend_from_slice(format!("\\u{:04x}", c as u32).as_bytes()),
            c => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// Canonical JSON array of `tracks`.
pub fn to_json(tracks: &[AudioTrack]) -> Vec<u8> {
    let mut out = vec![b'['];
    for (index, t) in tracks.iter().enumerate() {
        if index != 0 {
            out.push(b',');
        }
        out.extend_from_slice(b"{\"id\":");
        push_string(&mut out, &t.id);
        out.extend_from_slice(format!(",\"kind\":\"{}\",\"src\":", t.kind.name()).as_bytes());
        push_string(&mut out, &t.src);
        out.extend_from_slice(
            format!(
                ",\"start_tick\":{},\"trim_in_ms\":{}",
                t.start_tick, t.trim_in_ms
            )
            .as_bytes(),
        );
        if let Some(out_ms) = t.trim_out_ms {
            out.extend_from_slice(format!(",\"trim_out_ms\":{out_ms}").as_bytes());
        }
        out.extend_from_slice(
            format!(
                ",\"gain_db\":{},\"fade_in_ticks\":{},\"fade_out_ticks\":{},\"loop\":{}",
                t.gain_db, t.fade_in_ticks, t.fade_out_ticks, t.looped
            )
            .as_bytes(),
        );
        if let Some(duck) = &t.duck {
            out.extend_from_slice(b",\"duck\":{\"under\":");
            push_string(&mut out, &duck.under);
            out.extend_from_slice(format!(",\"amount_db\":{}}}", duck.amount_db).as_bytes());
        }
        out.push(b'}');
    }
    out.push(b']');
    out
}
