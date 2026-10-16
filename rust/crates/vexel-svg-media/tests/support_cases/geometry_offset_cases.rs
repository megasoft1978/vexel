use super::*;

#[test]
fn cubic_flatten_stops_and_late_invalid_geometry_keep_authored_run_offset() {
    struct StopAfter {
        left: usize,
        stop: Stop,
    }
    impl WorkControl for StopAfter {
        fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
            if self.left == 0 {
                Err(self.stop)
            } else {
                self.left -= 1;
                Ok(())
            }
        }
        fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
            if self.left == 0 {
                Err(self.stop)
            } else {
                self.left -= 1;
                Ok(())
            }
        }
    }
    use TextVerb::*;
    let commands = vec![
        command(Move, 0., 0.),
        TextCommand {
            verb: Cubic,
            values: [100., 100., -100., 100., 0., 0.],
        },
        command(Close, 0., 0.),
    ];
    let layout = TextLayout {
        glyphs: vec![TextGlyph {
            glyph_id: None,
            x: 0.,
            y: 0.,
            advance: 0.,
            commands: TableRange { first: 0, count: 3 },
        }],
        commands,
        advance_width: 0.,
        ascent: 0.,
        descent: 0.,
        leading: 0.,
    };
    let offset = 173;
    for stop in [Stop::Canceled, Stop::Limit] {
        let mut control = StopAfter { left: 10, stop };
        let error = vexel_svg_media::text_path_geometry_at(
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
            offset,
            &mut control,
        )
        .unwrap_err();
        let kind = if stop == Stop::Canceled {
            ErrorKind::Canceled
        } else {
            ErrorKind::Limit(LimitKind::Work)
        };
        assert_eq!((error.kind, error.offset), (kind, offset));
    }
    let mut invalid = TextLayout {
        glyphs: layout
            .glyphs
            .iter()
            .map(|g| TextGlyph {
                glyph_id: g.glyph_id,
                x: g.x,
                y: g.y,
                advance: g.advance,
                commands: g.commands,
            })
            .collect(),
        commands: layout
            .commands
            .iter()
            .map(|c| {
                let mut c = *c;
                c.values[0] = f64::INFINITY;
                c
            })
            .collect(),
        advance_width: layout.advance_width,
        ascent: layout.ascent,
        descent: layout.descent,
        leading: layout.leading,
    };
    invalid.commands[0].values[0] = 0.;
    let error = vexel_svg_media::text_path_geometry_at(
        &invalid,
        b"A",
        &[],
        &[],
        &[],
        &[],
        [0., 0.],
        0.,
        0.,
        vexel_geometry::affine_identity(),
        offset,
        &mut Run,
    )
    .unwrap_err();
    assert_eq!((error.kind, error.offset), (ErrorKind::Number, offset));
    invalid.commands[1].values[0] = f64::MAX;
    let error = vexel_svg_media::text_path_geometry_at(
        &invalid,
        b"A",
        &[],
        &[],
        &[],
        &[],
        [0., 0.],
        0.,
        0.,
        vexel_geometry::affine_identity(),
        offset,
        &mut Run,
    )
    .unwrap_err();
    assert_eq!((error.kind, error.offset), (ErrorKind::Number, offset));
}
