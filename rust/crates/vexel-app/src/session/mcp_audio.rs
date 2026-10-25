//! `audio_add_track`, `audio_set` and `audio_list`: timeline audio tracks
//! stored in the native timeline (so they save, undo and redo with it).
//!
//! A track's source is stored as a path relative to the directory of the saved
//! document. Every use re-resolves it through the grant policy: the file must
//! be a regular file, granted, and still inside the document directory after
//! symlinks are resolved. Files are size- and header-checked before use.

use super::mcp::McpOutcome;
use crate::preview::native_timeline::settings;
use crate::project_session::{NativeCommandError, ProjectTimelineError};
use crate::session::AppDocument;
use std::ffi::OsStr;
use std::fs::File;
use std::io::Read;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Path, PathBuf};
use vexel_mcp_access::AccessPolicy;
use vexel_timeline::audio::{AudioKind, AudioTrack, Duck, MAX_AUDIO_TRACKS, valid_relative_path};
use vexel_timeline::audio_codec::{self, AudioError, MAX_FILE_BYTES, Pcm};
use vexel_timeline::audio_mix::MixError;
use vexel_timeline::{EditLimits, Error as TimelineError};

#[derive(Clone, Debug, PartialEq)]
pub struct DuckSpec {
    pub under: String,
    pub amount_db: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NewTrack {
    pub id: String,
    pub kind: AudioKind,
    pub path: String,
    pub start_tick: u32,
    pub trim_in_ms: u32,
    pub trim_out_ms: Option<u32>,
    pub gain_db: f64,
    pub fade_in_ticks: u32,
    pub fade_out_ticks: u32,
    pub looped: bool,
    pub duck: Option<DuckSpec>,
}

/// Changes to one track; `None` leaves a field alone.
#[derive(Clone, Debug, PartialEq)]
pub struct SetTrack {
    pub id: String,
    pub remove: bool,
    pub gain_db: Option<f64>,
    pub fade_in_ticks: Option<u32>,
    pub fade_out_ticks: Option<u32>,
    pub start_tick: Option<u32>,
    pub trim_in_ms: Option<u32>,
    /// `Some(0)` clears the trim end.
    pub trim_out_ms: Option<u32>,
    pub looped: Option<bool>,
    pub duck: Option<DuckSpec>,
    pub clear_duck: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AudioOp {
    Add(Box<NewTrack>),
    Set(Box<SetTrack>),
    List,
}

const NEEDS_SAVE: &str = "audio paths are relative to the document; save it with save_as first";
const OUTSIDE: &str = "audio file must be inside the document's directory";
const NOT_FOUND: &str = "audio file not found";
const NOT_GRANTED: &str = "path not granted";
const REJECTED: &str = "audio edit rejected: trackId is 1-128 letters, digits, `_` or `-` (not voice, sfx or music); startTick below durationTicks; trimOutMs above trimInMs, both at most 600000; gainDb -120 to 24; fades at most durationTicks; duck.amountDb -60 to 0 and duck.under another track id or voice, sfx or music";

enum SourceError {
    Message(&'static str),
    Audio(AudioError),
}
impl SourceError {
    fn message(&self) -> &'static str {
        match self {
            Self::Message(m) => m,
            Self::Audio(e) => e.message(),
        }
    }
}

fn document_dir(app: &AppDocument) -> Result<PathBuf, &'static str> {
    let path = app.path().ok_or(NEEDS_SAVE)?;
    let parent = Path::new(OsStr::from_bytes(path))
        .parent()
        .ok_or(NEEDS_SAVE)?;
    std::fs::canonicalize(parent).map_err(|_| NEEDS_SAVE)
}

/// Resolves `input` (relative to the document, or absolute inside its
/// directory) to a granted regular file and returns its stored relative path.
fn locate(
    app: &AppDocument,
    policy: &AccessPolicy,
    input: &str,
) -> Result<(PathBuf, String), SourceError> {
    let base = document_dir(app).map_err(SourceError::Message)?;
    let requested = if input.starts_with('/') {
        PathBuf::from(input)
    } else if valid_relative_path(input) {
        base.join(input)
    } else {
        return Err(SourceError::Message(OUTSIDE));
    };
    let canonical = policy
        .resolve_existing(&requested)
        .map_err(|_| SourceError::Message(NOT_FOUND))?;
    if !policy.path_allowed(&canonical) {
        return Err(SourceError::Message(NOT_GRANTED));
    }
    let rel = canonical
        .strip_prefix(&base)
        .ok()
        .and_then(Path::to_str)
        .filter(|r| valid_relative_path(r))
        .ok_or(SourceError::Message(OUTSIDE))?
        .to_owned();
    Ok((canonical, rel))
}

#[cfg(target_os = "macos")]
const O_NOFOLLOW: i32 = 0x100;
#[cfg(not(target_os = "macos"))]
const O_NOFOLLOW: i32 = 0x20000;

/// Opens without following a final symlink and reads only a regular file, so
/// the file checked is the file read (no path re-resolution after the policy).
fn read_bounded(path: &Path) -> Result<Vec<u8>, SourceError> {
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(O_NOFOLLOW)
        .open(path)
        .map_err(|_| SourceError::Message(NOT_FOUND))?;
    let meta = file
        .metadata()
        .map_err(|_| SourceError::Message(NOT_FOUND))?;
    if !meta.is_file() {
        return Err(SourceError::Message(NOT_FOUND));
    }
    let len = meta.len();
    if len > MAX_FILE_BYTES as u64 {
        return Err(SourceError::Audio(AudioError::TooLarge));
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(len as usize)
        .map_err(|_| SourceError::Audio(AudioError::Alloc))?;
    file.take(MAX_FILE_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| SourceError::Message(NOT_FOUND))?;
    Ok(bytes)
}

/// Decodes a stored track's source for the export mixdown.
pub(crate) fn load_track(
    app: &AppDocument,
    policy: &AccessPolicy,
    track: &AudioTrack,
) -> Result<Pcm, MixError> {
    let (path, _) = locate(app, policy, &track.src).map_err(|_| MixError::Source)?;
    let bytes = read_bounded(&path).map_err(|e| match e {
        SourceError::Audio(a) => MixError::Audio(a),
        SourceError::Message(_) => MixError::Source,
    })?;
    audio_codec::decode(&bytes).map_err(MixError::Audio)
}

/// The document's audio tracks (empty without a readable native timeline).
pub(crate) fn tracks_of(app: &AppDocument) -> Vec<AudioTrack> {
    app.native_timeline_candidate(settings(), EditLimits::default())
        .ok()
        .and_then(|c| c.view().ok().map(|v| v.audio_tracks()))
        .unwrap_or_default()
}

fn commit(app: &mut AppDocument, tracks: &[AudioTrack]) -> Result<(), &'static str> {
    app.edit_timeline(settings(), EditLimits::default(), |c| {
        c.set_audio_tracks(tracks)
    })
    .map(|_| ())
    .map_err(|failure| match failure {
        NativeCommandError::Operation(ProjectTimelineError::Edit(f)) => match f.error {
            TimelineError::Limit => "audio edit rejected: at most 64 audio tracks",
            TimelineError::Duplicate => "audio edit rejected: trackId already exists",
            TimelineError::InvalidReference => {
                "audio edit rejected: duck.under must name another audio track or voice, sfx, music, and a track that others duck under cannot be removed"
            }
            _ => REJECTED,
        },
        _ => "audio edit rejected: the document has no editable native timeline",
    })
}

fn json_string(text: &str) -> String {
    let mut out = String::from("\"");
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn track_json(t: &AudioTrack) -> String {
    let mut out = format!(
        "{{\"trackId\":{},\"kind\":\"{}\",\"path\":{},\"startTick\":{},\"trimInMs\":{}",
        json_string(&t.id),
        t.kind.name(),
        json_string(&t.src),
        t.start_tick,
        t.trim_in_ms
    );
    if let Some(o) = t.trim_out_ms {
        out.push_str(&format!(",\"trimOutMs\":{o}"));
    }
    out.push_str(&format!(
        ",\"gainDb\":{},\"fadeInTicks\":{},\"fadeOutTicks\":{},\"loop\":{}",
        t.gain_db, t.fade_in_ticks, t.fade_out_ticks, t.looped
    ));
    if let Some(d) = &t.duck {
        out.push_str(&format!(
            ",\"duck\":{{\"under\":{},\"amountDb\":{}}}",
            json_string(&d.under),
            d.amount_db
        ));
    }
    out.push('}');
    out
}

fn duck(spec: DuckSpec) -> Duck {
    Duck {
        under: spec.under,
        amount_db: spec.amount_db,
    }
}

pub(crate) fn apply(
    app: &mut AppDocument,
    policy: &AccessPolicy,
    op: AudioOp,
) -> Result<McpOutcome, &'static str> {
    let mut tracks = tracks_of(app);
    match op {
        AudioOp::List => {
            let items: Vec<String> = tracks.iter().map(track_json).collect();
            Ok(McpOutcome::Reply(format!(
                "{{\"count\":{},\"max\":{MAX_AUDIO_TRACKS},\"tracks\":[{}]}}",
                items.len(),
                items.join(",")
            )))
        }
        AudioOp::Add(new) => {
            if tracks.iter().any(|t| t.id == new.id) {
                return Err("audio edit rejected: trackId already exists");
            }
            if tracks.len() >= MAX_AUDIO_TRACKS {
                return Err("audio edit rejected: at most 64 audio tracks");
            }
            let (path, rel) = locate(app, policy, &new.path).map_err(|e| e.message())?;
            let bytes = read_bounded(&path).map_err(|e| e.message())?;
            let info = audio_codec::probe(&bytes).map_err(AudioError::message)?;
            tracks.push(AudioTrack {
                id: new.id.clone(),
                kind: new.kind,
                src: rel.clone(),
                start_tick: new.start_tick,
                trim_in_ms: new.trim_in_ms,
                trim_out_ms: new.trim_out_ms.filter(|o| *o != 0),
                gain_db: new.gain_db,
                fade_in_ticks: new.fade_in_ticks,
                fade_out_ticks: new.fade_out_ticks,
                looped: new.looped,
                duck: new.duck.map(duck),
            });
            commit(app, &tracks)?;
            let millis = info.frames as u64 * 1000 / u64::from(info.rate);
            Ok(McpOutcome::Reply(format!(
                "{{\"ok\":true,\"trackId\":{},\"path\":{},\"format\":\"{}\",\"sampleRate\":{},\"channels\":{},\"durationMs\":{millis}}}",
                json_string(&new.id),
                json_string(&rel),
                info.format,
                info.rate,
                info.channels
            )))
        }
        AudioOp::Set(set) => {
            let index = tracks
                .iter()
                .position(|t| t.id == set.id)
                .ok_or("audio_set trackId is not an audio track; read ids with audio_list")?;
            if set.remove {
                tracks.remove(index);
            } else {
                let t = &mut tracks[index];
                t.gain_db = set.gain_db.unwrap_or(t.gain_db);
                t.fade_in_ticks = set.fade_in_ticks.unwrap_or(t.fade_in_ticks);
                t.fade_out_ticks = set.fade_out_ticks.unwrap_or(t.fade_out_ticks);
                t.start_tick = set.start_tick.unwrap_or(t.start_tick);
                t.trim_in_ms = set.trim_in_ms.unwrap_or(t.trim_in_ms);
                if let Some(out) = set.trim_out_ms {
                    t.trim_out_ms = (out != 0).then_some(out);
                }
                t.looped = set.looped.unwrap_or(t.looped);
                if set.clear_duck {
                    t.duck = None;
                }
                if let Some(d) = set.duck {
                    t.duck = Some(duck(d));
                }
            }
            commit(app, &tracks)?;
            Ok(McpOutcome::Reply(format!(
                "{{\"ok\":true,\"count\":{}}}",
                tracks.len()
            )))
        }
    }
}
