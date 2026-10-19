//! File-panel adapter for native drawing-track import effects.

use super::{NativeAnimationError, NativeTimelineEffect};
use crate::animation::native_drawing::SvgGeometry;
use crate::compose::{ComposeError, MAX_SVG_BYTES, open_path, svg_viewport};
use crate::file_ui::FileUi;
use crate::project_session::ProjectTimelineError;
use std::path::Path;
use std::sync::Arc;
use vexel_platform_apple::{PanelBackend, PanelResult};
use vexel_project::MAX_ASSET_NAME_BYTES;
use vexel_timeline::{EditLimits, EmptyTimelineSettings};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ImportResult {
    Imported,
    Cancelled,
    Failed,
}

/// Runs a file selection and commits the asset/cel/track as one journal edit.
/// The lifecycle barrier also keeps recovery snapshots from observing a
/// partially applied import.
pub fn handle<B: PanelBackend>(
    files: &mut FileUi<B>,
    effect: &NativeTimelineEffect,
    settings: EmptyTimelineSettings,
    limits: EditLimits,
) -> ImportResult {
    let NativeTimelineEffect::ImportSvg { target_id, tick } = effect;
    files.with_lifecycle(|document, backend| {
        let path = match backend.open_panel(&["svg"]) {
            PanelResult::Cancelled => return ImportResult::Cancelled,
            PanelResult::Error => {
                backend.show_error("Import SVG", "Could not open the SVG file panel.");
                return ImportResult::Failed;
            }
            PanelResult::Path(path) => path,
        };
        let path = match std::str::from_utf8(&path) {
            Ok(path) => Path::new(path),
            Err(_) => {
                backend.show_error("Import SVG", "The selected path is not valid UTF-8.");
                return ImportResult::Failed;
            }
        };
        let opened = match open_path(path) {
            Ok(opened) => opened,
            Err(error) => {
                backend.show_error("Import SVG", &error.to_string());
                return ImportResult::Failed;
            }
        };
        if opened.bytes.len() > MAX_SVG_BYTES {
            backend.show_error(
                "Import SVG",
                "The selected SVG exceeds the supported size limit.",
            );
            return ImportResult::Failed;
        }

        let source = match document.source_bytes() {
            Ok(source) => source,
            Err(error) => {
                backend.show_error("Import SVG", &error.user_message());
                return ImportResult::Failed;
            }
        };
        let viewport = match svg_viewport(Arc::new(source), &|| false) {
            Ok(viewport) => viewport,
            Err(error) => {
                report_compose_error(backend, error);
                return ImportResult::Failed;
            }
        };
        let geometry = SvgGeometry {
            width: f64::from(viewport.output_width),
            height: f64::from(viewport.output_height),
            view_box: [
                viewport.view_box.x,
                viewport.view_box.y,
                viewport.view_box.width,
                viewport.view_box.height,
            ],
        };

        let candidate = match document.native_timeline_candidate(settings, limits) {
            Ok(candidate) => candidate,
            Err(error) => {
                let error = NativeAnimationError::from(error);
                backend.show_error(
                    "Import SVG",
                    &format!("Could not prepare native timeline: {error:?}"),
                );
                return ImportResult::Failed;
            }
        };
        let (asset_name, track_id, track_name) = match names(&candidate) {
            Ok(names) => names,
            Err(error) => {
                backend.show_error(
                    "Import SVG",
                    &format!("Could not prepare native timeline: {error:?}"),
                );
                return ImportResult::Failed;
            }
        };
        let result = document.edit_timeline(settings, limits, |candidate| {
            candidate.import_drawing_track(
                &asset_name,
                b"image/svg+xml",
                &opened.bytes,
                geometry,
                &track_id,
                target_id.as_bytes(),
                &track_name,
                *tick,
                b"Imported",
            )
        });
        match result {
            Ok(true) => ImportResult::Imported,
            Ok(false) => ImportResult::Failed,
            Err(error) => {
                backend.show_error("Import SVG", &format!("Could not import SVG: {error:?}"));
                ImportResult::Failed
            }
        }
    })
}

fn report_compose_error(backend: &dyn PanelBackend, error: ComposeError) {
    backend.show_error(
        "Import SVG",
        &format!("Could not read the current scene geometry: {error}"),
    );
}

fn names(
    candidate: &crate::project_session::ProjectTimelineCandidate,
) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), ProjectTimelineError> {
    let view = candidate.view()?;
    let mut asset_suffix = 1u32;
    let asset_name = loop {
        let asset_name = if asset_suffix == 1 {
            b"drawing.svg".to_vec()
        } else {
            format!("drawing-{asset_suffix}.svg").into_bytes()
        };
        if asset_name.len() <= MAX_ASSET_NAME_BYTES
            && !candidate
                .assets()
                .iter()
                .any(|asset| asset.name == asset_name)
        {
            break asset_name;
        }
        asset_suffix = asset_suffix
            .checked_add(1)
            .ok_or(ProjectTimelineError::Limit)?;
    };
    let mut track_suffix = 1u32;
    let track_id = loop {
        let track_id = format!("native-drawing-{track_suffix}").into_bytes();
        if !(0..view.track_count()).any(|index| {
            view.track_at(index)
                .is_some_and(|track| track.id == track_id)
        }) {
            break track_id;
        }
        track_suffix = track_suffix
            .checked_add(1)
            .ok_or(ProjectTimelineError::Limit)?;
    };
    let track_name = if track_suffix == 1 {
        b"Imported drawing".to_vec()
    } else {
        format!("Imported drawing {track_suffix}").into_bytes()
    };
    Ok((asset_name, track_id, track_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project_session::ProjectTimelineCandidate;
    use vexel_timeline::LoopMode;

    fn settings() -> EmptyTimelineSettings {
        EmptyTimelineSettings {
            ticks_per_second: 24,
            duration_ticks: 240,
            fps_numerator: 24,
            fps_denominator: 1,
            loop_mode: LoopMode::Once,
            onion_before: 0,
            onion_after: 0,
            onion_opacity: 0.5,
        }
    }

    #[test]
    fn generated_names_avoid_existing_asset_and_track_ids() {
        let initial = ProjectTimelineCandidate::open(
            b"",
            vec![b"shape".to_vec()],
            Vec::new(),
            settings(),
            EditLimits::default(),
        )
        .unwrap();
        let occupied = initial
            .add_property_track(
                b"native-drawing-1",
                b"shape",
                b"opacity",
                b"scalar",
                b"linear",
                b"1",
                b"Drawing",
            )
            .unwrap();
        let (asset, id, label) = names(&occupied).unwrap();
        assert_eq!(asset, b"drawing.svg");
        assert_eq!(id, b"native-drawing-2");
        assert_eq!(label, b"Imported drawing 2");
    }

    #[test]
    fn generated_names_skip_asset_collisions_independently() {
        const SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><rect width="1" height="1"/></svg>"#;
        let initial = ProjectTimelineCandidate::open(
            b"",
            vec![b"shape".to_vec()],
            Vec::new(),
            settings(),
            EditLimits::default(),
        )
        .unwrap();
        let occupied = initial
            .import_drawing_track(
                b"drawing.svg",
                b"image/svg+xml",
                SVG,
                SvgGeometry {
                    width: 1.0,
                    height: 1.0,
                    view_box: [0.0, 0.0, 1.0, 1.0],
                },
                b"existing-track",
                b"shape",
                b"Existing drawing",
                0,
                b"Existing",
            )
            .unwrap();
        let (asset, id, label) = names(&occupied).unwrap();
        assert_eq!(asset, b"drawing-2.svg");
        assert_eq!(id, b"native-drawing-1");
        assert_eq!(label, b"Imported drawing");
    }

    struct Backend {
        result: PanelResult,
    }

    impl PanelBackend for Backend {
        fn open_panel(&self, _: &[&str]) -> PanelResult {
            self.result.clone()
        }

        fn save_panel(&self, _: &str) -> PanelResult {
            PanelResult::Cancelled
        }

        fn confirm_unsaved(
            &self,
            _: &vexel_platform_apple::UnsavedText<'_>,
        ) -> vexel_platform_apple::UnsavedChoice {
            vexel_platform_apple::UnsavedChoice::Cancel
        }

        fn show_error(&self, _: &str, _: &str) {}
    }

    #[test]
    fn cancelling_file_selection_leaves_document_and_history_unchanged() {
        let mut files = FileUi::with_size(
            Backend {
                result: PanelResult::Cancelled,
            },
            (64, 64),
        )
        .unwrap();
        let effect = NativeTimelineEffect::ImportSvg {
            target_id: "shape".to_owned(),
            tick: 0,
        };
        assert_eq!(
            handle(&mut files, &effect, settings(), EditLimits::default()),
            ImportResult::Cancelled
        );
        assert_eq!(files.document().state_revision(), 0);
        assert!(!files.document().is_dirty());
        assert!(!files.document().can_undo());
    }
}
