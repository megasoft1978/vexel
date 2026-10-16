use vexel_base::{Stop, WorkControl, WorkKind};
use vexel_scene::{
    ErrorKind, LimitKind, TableRange, TextCommand, TextGlyph, TextLayout, TextState, TextVerb,
};
use vexel_svg_media::{normalize_text, text_path_geometry_at};
use vexel_xml::XmlSpan;

struct StopGeometry {
    at_checkpoint: bool,
    stop: Stop,
    checkpoints: usize,
    charges: usize,
}
impl WorkControl for StopGeometry {
    fn checkpoint(&mut self) -> Result<(), Stop> {
        self.checkpoints += 1;
        if self.at_checkpoint {
            Err(self.stop)
        } else {
            Ok(())
        }
    }
    fn charge(&mut self, kind: WorkKind, amount: u64) -> Result<(), Stop> {
        assert_eq!((kind, amount), (WorkKind::Media, 1));
        self.charges += 1;
        Err(self.stop)
    }
}

fn assert_outline_stop_offset(at_checkpoint: bool) {
    let layout = TextLayout {
        glyphs: vec![TextGlyph {
            glyph_id: None,
            x: 0.,
            y: 0.,
            advance: 1.,
            commands: TableRange { first: 0, count: 1 },
        }],
        commands: vec![TextCommand {
            verb: TextVerb::Move,
            values: [0.; 6],
        }],
        advance_width: 1.,
        ascent: 1.,
        descent: 0.,
        leading: 0.,
    };
    for (stop, kind) in [
        (Stop::Canceled, ErrorKind::Canceled),
        (Stop::Limit, ErrorKind::Limit(LimitKind::Work)),
    ] {
        let mut control = StopGeometry {
            at_checkpoint,
            stop,
            checkpoints: 0,
            charges: 0,
        };
        let error = text_path_geometry_at(
            &layout,
            b"A",
            &[],
            &[],
            &[],
            &[],
            [0., 0.],
            0.,
            0.,
            vexel_geometry::affine_identity(),
            173,
            &mut control,
        )
        .unwrap_err();
        assert_eq!((error.kind, error.offset), (kind, 173));
        assert_eq!(control.checkpoints, 1);
        assert_eq!(control.charges, usize::from(!at_checkpoint));
    }
}

#[test]
fn glyph_checkpoint_cancellation_and_work_limit_keep_authored_offset() {
    assert_outline_stop_offset(true);
}

#[test]
fn outline_command_charge_cancellation_and_work_limit_keep_authored_offset() {
    assert_outline_stop_offset(false);
}

#[test]
fn public_normalization_rejects_xml_nul_without_changing_state_in_both_modes() {
    for preserve in [false, true] {
        let mut state = TextState {
            cursor_x: 12.,
            cursor_y: -7.,
            text_bytes: 23,
            glyph_count: 3,
            pending_space: true,
            has_position: true,
        };
        let before = state;
        let error = normalize_text(
            b"xxxx\0",
            XmlSpan {
                offset: 4,
                length: 1,
            },
            preserve,
            &mut state,
        )
        .unwrap_err();
        assert_eq!((error.kind, error.offset), (ErrorKind::Unsupported, 4));
        assert_eq!(state.cursor_x, before.cursor_x);
        assert_eq!(state.cursor_y, before.cursor_y);
        assert_eq!(state.text_bytes, before.text_bytes);
        assert_eq!(state.glyph_count, before.glyph_count);
        assert_eq!(state.pending_space, before.pending_space);
        assert_eq!(state.has_position, before.has_position);
    }
}
