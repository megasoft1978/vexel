//! Pure timeline data: tracks and key times derived read-only from the
//! document's SMIL animation, plus the playback state shown by the panel.

use super::layout::Hit;
use crate::animation::driver::Driver;
use crate::animation::playback::LoopClock;
use crate::chrome::{NO_ANIMATION, NO_DOCUMENT_TO_ANIMATE};
use crate::motion::TOOLTIP_DELAY_NS;
use std::sync::Arc;
use vexel_svg_animation::{Track, attribute_name, parse_owned};
use vexel_xml::{XmlLimits, tokenize};

/// Source bytes the track reader accepts (same bound as the animation decoder).
const SOURCE_LIMIT: usize = 4 << 20;

/// One animated property of an element with its key times in seconds.
#[derive(Clone, Debug, PartialEq)]
pub struct PropTrack {
    pub name: String,
    /// Sorted, inside `[0, loop]`.
    pub keys: Vec<f64>,
}

/// One animated element: its properties, in document order.
#[derive(Clone, Debug, PartialEq)]
pub struct ElementTrack {
    pub name: String,
    pub props: Vec<PropTrack>,
}

impl ElementTrack {
    /// Union of the property key times (sorted, de-duplicated).
    pub fn summary_keys(&self) -> Vec<f64> {
        let mut all: Vec<f64> = self
            .props
            .iter()
            .flat_map(|p| p.keys.iter().copied())
            .collect();
        all.sort_by(f64::total_cmp);
        all.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        all
    }
}

/// A key in the model: element, property (`None` = the element's summary row), index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyRef {
    pub element: usize,
    pub prop: Option<usize>,
    pub index: usize,
}

/// One visible row of the track list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Row {
    Element(usize),
    Prop(usize, usize),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimelineModel {
    /// Revision the tracks were read for.
    pub revision: Option<u64>,
    pub open: bool,
    pub loop_seconds: f64,
    pub time: f64,
    pub playing: bool,
    pub looping: bool,
    pub elements: Vec<ElementTrack>,
    /// The driver clock at the last sync (drives the tooltip delay).
    pub now: f64,
    /// The disabled control the pointer rests on and since when (clock seconds).
    pub tip: Option<(Hit, f64)>,
}

impl TimelineModel {
    /// Reads the tracks of `source` for `revision` (loop length from the driver).
    pub fn set_document(&mut self, revision: u64, source: &[u8], loop_seconds: f64) {
        self.revision = Some(revision);
        self.open = true;
        self.loop_seconds = loop_seconds;
        self.elements = if loop_seconds > 0.0 {
            read_tracks(source, loop_seconds)
        } else {
            Vec::new()
        };
    }

    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Copies the driver's playback state (cheap; called every frame).
    /// True when something the panel shows changed.
    pub fn sync_playback<C: LoopClock>(&mut self, driver: &Driver<C>) -> bool {
        let next = (
            driver.loop_seconds(),
            driver.time(),
            driver.is_playing(),
            driver.is_looping(),
        );
        let tip_before = self.tip_ready();
        let changed = next != (self.loop_seconds, self.time, self.playing, self.looping);
        (self.loop_seconds, self.time, self.playing, self.looping) = next;
        self.now = driver.playback().clock().now_seconds();
        changed || tip_before != self.tip_ready()
    }

    /// The disabled control whose tooltip is showing: hovered for the tooltip
    /// delay of the polish layer.
    pub fn tip_ready(&self) -> Option<Hit> {
        let (hit, since) = self.tip?;
        (self.now - since >= TOOLTIP_DELAY_NS as f64 / 1e9).then_some(hit)
    }

    /// The text shown instead of tracks, or `None` when there are tracks.
    pub fn empty_state(&self) -> Option<String> {
        if !self.open {
            Some(NO_DOCUMENT_TO_ANIMATE.to_owned())
        } else if self.loop_seconds <= 0.0 {
            Some(NO_ANIMATION.to_owned())
        } else if self.elements.is_empty() {
            Some("No animated properties".to_owned())
        } else {
            None
        }
    }

    /// Visible rows given per-element collapse flags.
    pub fn rows(&self, collapsed: &[bool]) -> Vec<Row> {
        let mut rows = Vec::new();
        for (e, element) in self.elements.iter().enumerate() {
            rows.push(Row::Element(e));
            if !collapsed.get(e).copied().unwrap_or(false) {
                rows.extend((0..element.props.len()).map(|p| Row::Prop(e, p)));
            }
        }
        rows
    }

    /// Key times of a row.
    pub fn row_keys(&self, row: Row) -> Vec<f64> {
        match row {
            Row::Element(e) => self.elements[e].summary_keys(),
            Row::Prop(e, p) => self.elements[e].props[p].keys.clone(),
        }
    }
}

fn read_tracks(source: &[u8], loop_seconds: f64) -> Vec<ElementTrack> {
    let (tokens, result) = tokenize(
        source,
        XmlLimits {
            input_bytes: SOURCE_LIMIT,
            ..XmlLimits::default()
        },
    );
    if result.diagnostic.category != vexel_xml::XmlError::None {
        return Vec::new();
    }
    let Ok(doc) = parse_owned(&Arc::new(source.to_vec()), &tokens) else {
        return Vec::new();
    };
    let mut elements: Vec<(Vec<u8>, ElementTrack)> = Vec::new();
    for track in doc.tracks().iter().filter(|t| t.supported) {
        let (key, name) = element_name(source, track);
        let prop = PropTrack {
            name: attribute_name(track.attribute).to_owned(),
            keys: key_times(track, loop_seconds),
        };
        match elements.iter_mut().find(|(k, _)| *k == key) {
            Some((_, e)) => e.props.push(prop),
            None => elements.push((
                key,
                ElementTrack {
                    name,
                    props: vec![prop],
                },
            )),
        }
    }
    elements.into_iter().map(|(_, e)| e).collect()
}

/// Grouping key and display name of the element a track animates: its `id`
/// attribute, else its tag name.
fn element_name(source: &[u8], track: &Track) -> (Vec<u8>, String) {
    let tag = track.target.slice(source).unwrap_or(&[]);
    let key = format!("{}:{}", track.target.offset, track.target.length).into_bytes();
    let text = String::from_utf8_lossy(tag);
    let name = attribute(&text, "id")
        .or_else(|| {
            let name = text.trim_start_matches('<');
            let end = name.find(|c: char| c.is_whitespace() || c == '>' || c == '/');
            Some(name[..end.unwrap_or(name.len())].to_owned())
        })
        .filter(|n| !n.is_empty());
    (key, name.unwrap_or_else(|| "element".to_owned()))
}

/// Value of ` name="..."` / `name='...'` inside a start tag.
fn attribute(tag: &str, name: &str) -> Option<String> {
    for quote in ['"', '\''] {
        let needle = format!(" {name}={quote}");
        if let Some(at) = tag.find(&needle) {
            let rest = &tag[at + needle.len()..];
            return rest.find(quote).map(|end| rest[..end].to_owned());
        }
    }
    None
}

/// Absolute key times of one period of the track, clamped into the loop.
fn key_times(track: &Track, loop_seconds: f64) -> Vec<f64> {
    let begin = if track.begin_seconds.is_finite() {
        track.begin_seconds.max(0.0)
    } else {
        0.0
    };
    let duration = if track.duration_seconds.is_finite() {
        track.duration_seconds.max(0.0)
    } else {
        0.0
    };
    let mut times: Vec<f64> = track
        .keys
        .iter()
        .map(|k| (begin + k.time.clamp(0.0, 1.0) * duration).clamp(0.0, loop_seconds))
        .collect();
    times.sort_by(f64::total_cmp);
    times.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    times
}
