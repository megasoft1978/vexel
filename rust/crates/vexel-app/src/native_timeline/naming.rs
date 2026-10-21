//! Human readable track labels derived from the track's property, name and
//! target. The UI never shows raw ids when something friendlier exists.

use super::NativeTrackKind;
use super::model::Track;

/// `opacity`, `translate_x`, `stroke-width` -> lower case words.
pub fn property_label(property: &str) -> String {
    let words = property.trim().replace(['_', '-', '.'], " ");
    match words.as_str() {
        "x" => "position x".to_owned(),
        "y" => "position y".to_owned(),
        _ => words.to_lowercase(),
    }
}

/// True for ids the app or a script generated (`t1`, `track-3`): not a name.
fn generic(name: &str, id: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n == id {
        return true;
    }
    let lower = n.to_lowercase();
    let digits =
        lower.trim_start_matches(|c: char| c.is_ascii_alphabetic() || c == '-' || c == '_');
    !digits.is_empty()
        && digits.chars().all(|c| c.is_ascii_digit())
        && lower.len() - digits.len() <= 6
}

/// Primary row title: the property for property tracks, else the track name.
/// `index` is the zero-based row, used only for the `Track N` fallback.
pub fn track_title(track: &Track, index: usize) -> String {
    let named = !generic(&track.name, &track.id);
    match track.kind {
        NativeTrackKind::Property if !track.property.trim().is_empty() => {
            property_label(&track.property)
        }
        _ if named => track.name.trim().to_owned(),
        NativeTrackKind::Drawing => format!("Drawing {}", index + 1),
        NativeTrackKind::Property => format!("Track {}", index + 1),
    }
}

/// Secondary label: the element the track animates (`#hero`), when known.
pub fn track_subtitle(track: &Track) -> Option<String> {
    let target = track.target.trim();
    (!target.is_empty()).then(|| format!("#{target}"))
}

/// Short key tooltip-ish caption for the key bar.
pub fn ease_name(ease: &str) -> &'static str {
    match ease {
        "linear" => "Linear",
        "ease_in" => "Ease in",
        "ease_out" => "Ease out",
        "ease_in_out" => "Ease in-out",
        "step" => "Step",
        e if super::bezier::parse(e).is_some() => "Custom bezier",
        _ => "Linear",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn track(id: &str, name: &str, property: &str, kind: NativeTrackKind) -> Track {
        Track {
            id: id.into(),
            name: name.into(),
            target: "hero".into(),
            property: property.into(),
            kind,
            visible: true,
            selected: false,
            items: Vec::new(),
        }
    }

    #[test]
    fn property_tracks_show_the_property_not_the_id() {
        let t = track("t1", "T1", "translate_x", NativeTrackKind::Property);
        assert_eq!(track_title(&t, 0), "translate x");
        assert_eq!(track_subtitle(&t).as_deref(), Some("#hero"));
        let t = track("o", "Opacity", "opacity", NativeTrackKind::Property);
        assert_eq!(track_title(&t, 3), "opacity");
    }

    #[test]
    fn missing_names_fall_back_to_numbered_tracks() {
        assert_eq!(
            track_title(&track("t1", "t1", "", NativeTrackKind::Property), 0),
            "Track 1"
        );
        assert_eq!(
            track_title(&track("t7", "T7", "", NativeTrackKind::Drawing), 6),
            "Drawing 7"
        );
        assert_eq!(
            track_title(&track("d", "Body", "", NativeTrackKind::Drawing), 0),
            "Body"
        );
    }

    #[test]
    fn labels_are_ascii() {
        for e in [
            "linear",
            "ease_in",
            "ease_out",
            "ease_in_out",
            "step",
            "bezier:0,0,1,1",
        ] {
            assert!(ease_name(e).is_ascii());
        }
    }
}
