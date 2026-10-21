//! Lane u1: the extended inspector (opacity, transform, dash, gradient fill).
//! Controls are hit-tested through the layout, edit the selected shape through
//! the session (one undo step each) and are drawn from the same rects.
mod ui_wiring_support;
use ui_wiring_support::*;
use vexel_app::chrome::{
    Dash, FillKind, Inspector, ShapeExt, ShapeProps, TransformView, normalize_degrees,
};
use vexel_app::layout::{
    EXT_COUNT, EXT_END, EXT_FIRST, ExtControl, ExtRow, SECTION_BODY_H, tooltip,
};
use vexel_app::model::{Mode, Model, Target};
use vexel_app::render::render;
use vexel_app::session::EditAction;
use vexel_app::toolbar_actions;
use vexel_app::tools::ToolController;
use vexel_v1_document::{TransformSpec, V1Document};

const ID: &str = "r1";

/// A shell with one selected, stroked rectangle (10,10)-(30,40).
fn rig() -> (Shell, ToolController) {
    let mut shell = Shell::new();
    shell.edit(ID, 10.0);
    shell
        .files
        .document_mut()
        .edit(|d| d.set_stroke(ID, "#000000", 2.0))
        .unwrap();
    let mut tools = ToolController::new();
    tools
        .select_id(shell.files.document().document(), ID)
        .unwrap();
    show_selection(&mut shell);
    (shell, tools)
}

/// What `from_session` does each tick: the inspector and extension rows.
fn show_selection(shell: &mut Shell) {
    shell.refresh();
    let ext = ShapeExt::read(shell.files.document().document(), ID);
    shell.model.chrome.state.ext = ext;
    shell.model.chrome.inspector = Inspector::Shape(ShapeProps {
        index: 0,
        fill: Some([255, 0, 0, 255]),
        stroke: Some([0, 0, 0, 255]),
        stroke_width: 2.0,
        transform: vexel_geometry::affine_identity(),
    });
    shell.model.ui.collapsed = [false; 5];
}

fn press(shell: &mut Shell, tools: &mut ToolController, control: ExtControl) -> bool {
    let consumed =
        toolbar_actions::inspector_edit(control.index(), &mut shell.model, &mut shell.files, tools);
    show_selection(shell);
    consumed
}

fn ext(shell: &Shell) -> ShapeExt {
    ShapeExt::read(shell.files.document().document(), ID).unwrap()
}

fn text(shell: &Shell) -> String {
    String::from_utf8(shell.source()).unwrap()
}

fn undo(shell: &mut Shell) -> bool {
    let changed = toolbar_actions::edit(
        &mut shell.model,
        shell.files.document_mut(),
        EditAction::Undo,
    );
    show_selection(shell);
    changed
}

fn redo(shell: &mut Shell) -> bool {
    let changed = toolbar_actions::edit(
        &mut shell.model,
        shell.files.document_mut(),
        EditAction::Redo,
    );
    show_selection(shell);
    changed
}

/// Presses `control`, then checks one undo restores the exact source and one
/// redo brings the edit back.
fn press_undoable(shell: &mut Shell, tools: &mut ToolController, control: ExtControl) -> String {
    let before = shell.source();
    assert!(press(shell, tools, control));
    let after = shell.source();
    assert_ne!(before, after, "{control:?} changed nothing");
    assert!(undo(shell));
    assert_eq!(shell.source(), before, "{control:?} undo");
    assert!(redo(shell));
    assert_eq!(shell.source(), after, "{control:?} redo");
    String::from_utf8(after).unwrap()
}

#[test]
fn control_indices_round_trip_and_stay_clear_of_the_swatch_range() {
    assert_eq!(EXT_FIRST, 21);
    let mut seen = Vec::new();
    for index in EXT_FIRST..EXT_END {
        let control = ExtControl::from_index(index).expect("every index maps");
        assert_eq!(control.index(), index);
        assert!(!control.tip().0.is_empty());
        assert!(tooltip(Target::SectionHeader(index)).is_some(), "{index}");
        seen.push(control);
    }
    assert_eq!(seen.len(), EXT_COUNT);
    assert_eq!(ExtControl::from_index(EXT_END), None);
    assert_eq!(ExtControl::from_index(20), None);
    assert!(tooltip(Target::SectionHeader(0)).is_none());
}

#[test]
fn every_control_is_hit_testable_inside_its_section_and_never_overlaps() {
    let (shell, _) = rig();
    let layout = shell.layout();
    let mut rects = Vec::new();
    for index in EXT_FIRST..EXT_END {
        let control = ExtControl::from_index(index).unwrap();
        let r = layout
            .ext_control_rect(control)
            .unwrap_or_else(|| panic!("{control:?} has a rect with all sections open"));
        assert_eq!(
            layout.hit_test((r.x + r.width / 2, r.y + r.height / 2)),
            Some(Target::SectionHeader(index)),
            "{control:?}"
        );
        let panel = layout.inspector.unwrap();
        assert!(r.x >= panel.x && r.x + r.width <= panel.x + panel.width);
        assert!(r.width >= 20 && r.height >= 20, "{control:?} target size");
        rects.push((control, r));
    }
    for (i, (a, ra)) in rects.iter().enumerate() {
        for (b, rb) in &rects[i + 1..] {
            let apart = ra.x + ra.width <= rb.x
                || rb.x + rb.width <= ra.x
                || ra.y + ra.height <= rb.y
                || rb.y + rb.height <= ra.y;
            assert!(apart, "{a:?} overlaps {b:?}");
        }
    }
    // Rows sit inside their section body.
    for (row, section) in [
        (ExtRow::Opacity, 0),
        (ExtRow::Angle, 1),
        (ExtRow::Dash, 2),
        (ExtRow::ScaleY, 3),
    ] {
        let r = layout.ext_row(row).unwrap();
        let body = layout.section_bodies[section];
        assert_eq!(body.height, SECTION_BODY_H[section]);
        assert!(r.y + r.height <= body.y + body.height);
    }
}

#[test]
fn controls_vanish_with_no_selection_or_a_collapsed_section() {
    let (mut shell, _) = rig();
    shell.model.ui.collapsed[3] = true;
    let layout = shell.layout();
    assert!(layout.ext_control_rect(ExtControl::RotateUp).is_none());
    assert!(layout.ext_control_rect(ExtControl::OpacityUp).is_some());
    shell.model.chrome.inspector = Inspector::NoSelection;
    let layout = shell.layout();
    assert!(layout.ext_control_rect(ExtControl::OpacityUp).is_none());
    assert!(layout.ext_control_rect(ExtControl::Dash(1)).is_none());
}

#[test]
fn opacity_steps_are_single_undoable_edits_and_clamp() {
    let (mut shell, mut tools) = rig();
    assert_eq!(ext(&shell).opacity, 1.0);
    assert_eq!(press(&mut shell, &mut tools, ExtControl::OpacityUp), true);
    assert_eq!(shell.model.status, "No change", "already fully opaque");
    let src = press_undoable(&mut shell, &mut tools, ExtControl::OpacityDown);
    assert!(src.contains("opacity=\"0.9\""), "{src}");
    assert_eq!(shell.model.status, "Redid change");
    assert_eq!(ext(&shell).opacity, 0.9);
    for _ in 0..12 {
        press(&mut shell, &mut tools, ExtControl::OpacityDown);
    }
    assert_eq!(ext(&shell).opacity, 0.0);
    assert!(text(&shell).contains("opacity=\"0\""));
    press(&mut shell, &mut tools, ExtControl::OpacityUp);
    assert_eq!(ext(&shell).opacity, 0.1);
}

#[test]
fn dash_presets_set_array_and_round_caps_and_none_removes() {
    let (mut shell, mut tools) = rig();
    assert_eq!(ext(&shell).dash, Dash::None);
    let src = press_undoable(&mut shell, &mut tools, ExtControl::Dash(1));
    assert!(src.contains("stroke-dasharray=\"8 6\""), "{src}");
    assert!(src.contains("stroke-linecap=\"round\""), "{src}");
    assert_eq!(ext(&shell).dash, Dash::Dashed);
    let src = press_undoable(&mut shell, &mut tools, ExtControl::Dash(2));
    assert!(src.contains("stroke-dasharray=\"0 4\""), "{src}");
    assert_eq!(ext(&shell).dash, Dash::Dotted);
    let src = press_undoable(&mut shell, &mut tools, ExtControl::Dash(0));
    assert!(!src.contains("stroke-dasharray"), "{src}");
    assert_eq!(ext(&shell).dash, Dash::None);
    press(&mut shell, &mut tools, ExtControl::Dash(0));
    assert_eq!(shell.model.status, "No change");
}

#[test]
fn rotation_and_scale_apply_about_the_bounds_centre_and_repeat() {
    let (mut shell, mut tools) = rig();
    let TransformView::Parts(p) = ext(&shell).transform else {
        panic!("identity decomposes");
    };
    assert_eq!((p.rotation, p.scale), (0.0, (1.0, 1.0)));
    assert_eq!(p.pivot, (20.0, 25.0), "bounds centre");
    let src = press_undoable(&mut shell, &mut tools, ExtControl::RotateUp);
    assert!(src.contains("transform=\"matrix("), "{src}");
    let TransformView::Parts(p) = ext(&shell).transform else {
        panic!("rotation decomposes");
    };
    assert_eq!(p.rotation, 15.0);
    assert_eq!(p.scale, (1.0, 1.0));
    assert!(
        (p.pivot.0 - 20.0).abs() < 1e-3 && (p.pivot.1 - 25.0).abs() < 1e-3,
        "{p:?}"
    );
    // The centre stays put under the matrix (rotation really is about it).
    let m = shell
        .files
        .document()
        .document()
        .get_transform_matrix(ID)
        .unwrap();
    let (cx, cy) = (
        m[0] * 20.0 + m[2] * 25.0 + m[4],
        m[1] * 20.0 + m[3] * 25.0 + m[5],
    );
    assert!((cx - 20.0).abs() < 1e-6 && (cy - 25.0).abs() < 1e-6);
    press(&mut shell, &mut tools, ExtControl::RotateUp);
    press(&mut shell, &mut tools, ExtControl::RotateDown);
    press(&mut shell, &mut tools, ExtControl::RotateDown);
    press(&mut shell, &mut tools, ExtControl::RotateDown);
    let TransformView::Parts(p) = ext(&shell).transform else {
        panic!("decomposes");
    };
    assert_eq!(p.rotation, -15.0);
    // Back to zero removes the transform entirely.
    press(&mut shell, &mut tools, ExtControl::RotateUp);
    assert!(!text(&shell).contains("transform="), "{}", text(&shell));
    // Scale.
    let src = press_undoable(&mut shell, &mut tools, ExtControl::ScaleXUp);
    assert!(src.contains("transform=\"matrix(1.1e0 "), "{src}");
    press(&mut shell, &mut tools, ExtControl::ScaleYDown);
    let TransformView::Parts(p) = ext(&shell).transform else {
        panic!("decomposes");
    };
    assert_eq!(p.scale, (1.1, 0.9));
    assert!(
        (p.pivot.0 - 20.0).abs() < 1e-3 && (p.pivot.1 - 25.0).abs() < 1e-3,
        "{p:?}"
    );
}

#[test]
fn scale_never_lands_on_zero_and_rotation_wraps() {
    assert_eq!(normalize_degrees(190.0), -170.0);
    assert_eq!(normalize_degrees(-180.0), 180.0);
    assert_eq!(normalize_degrees(720.0), 0.0);
    let (mut shell, mut tools) = rig();
    for _ in 0..9 {
        press(&mut shell, &mut tools, ExtControl::ScaleXDown);
    }
    let TransformView::Parts(p) = ext(&shell).transform else {
        panic!()
    };
    assert_eq!(p.scale.0, 0.1);
    press(&mut shell, &mut tools, ExtControl::ScaleXDown);
    let TransformView::Parts(p) = ext(&shell).transform else {
        panic!()
    };
    assert_eq!(p.scale.0, 0.1, "stops at the minimum instead of mirroring");
    assert_eq!(shell.model.status, "No change");
}

#[test]
fn skewed_transforms_show_custom_and_refuse_edits() {
    let (mut shell, mut tools) = rig();
    shell
        .files
        .document_mut()
        .edit(|d| {
            d.set_transform(
                ID,
                &TransformSpec {
                    skew_x_deg: 20.0,
                    ..TransformSpec::default()
                },
            )
        })
        .unwrap();
    show_selection(&mut shell);
    assert_eq!(ext(&shell).transform, TransformView::Custom);
    let before = shell.source();
    let cursor = shell.cursor();
    assert!(press(&mut shell, &mut tools, ExtControl::RotateUp));
    assert_eq!(shell.model.status, "Could not apply that change");
    assert_eq!(shell.source(), before);
    assert_eq!(shell.cursor(), cursor);
}

#[test]
fn gradient_fill_creates_applies_and_edits_stops_with_one_undo_each() {
    let (mut shell, mut tools) = rig();
    assert_eq!(ext(&shell).fill, FillKind::Solid);
    let solid = shell.source();
    let src = press_undoable(&mut shell, &mut tools, ExtControl::FillKind(1));
    assert!(src.contains("<linearGradient"), "{src}");
    assert!(src.contains("fill=\"url(#vexel-gradient-"), "{src}");
    let e = ext(&shell);
    assert_eq!(e.fill, FillKind::Linear);
    assert_eq!(e.angle, Some(0.0));
    assert_eq!(
        e.stops,
        Some([[255, 0, 0], [255, 255, 255]]),
        "from the solid fill"
    );
    // Same kind again is a no-op; the other kind swaps the gradient.
    press(&mut shell, &mut tools, ExtControl::FillKind(1));
    assert_eq!(shell.model.status, "No change");
    let src = press_undoable(&mut shell, &mut tools, ExtControl::FillKind(2));
    assert!(src.contains("<radialGradient"), "{src}");
    assert_eq!(ext(&shell).fill, FillKind::Radial);
    // Back to linear, then recolour both stops from the swatch row.
    press(&mut shell, &mut tools, ExtControl::FillKind(1));
    let src = press_undoable(
        &mut shell,
        &mut tools,
        ExtControl::GradStop { stop: 0, swatch: 5 },
    );
    assert!(src.contains("stop-color=\"#1e88e5\""), "{src}");
    press(
        &mut shell,
        &mut tools,
        ExtControl::GradStop { stop: 1, swatch: 4 },
    );
    assert_eq!(
        ext(&shell).stops,
        Some([[0x1e, 0x88, 0xe5], [0x43, 0xa0, 0x47]])
    );
    // Picking the colour a stop already has does nothing.
    press(
        &mut shell,
        &mut tools,
        ExtControl::GradStop { stop: 1, swatch: 4 },
    );
    assert_eq!(shell.model.status, "No change");
    // Back to solid takes the first stop's colour.
    let src = press_undoable(&mut shell, &mut tools, ExtControl::FillKind(0));
    assert!(src.contains("fill=\"#1e88e5\""), "{src}");
    assert_eq!(ext(&shell).fill, FillKind::Solid);
    assert_ne!(shell.source(), solid);
}

#[test]
fn gradient_angle_steps_rebuild_the_linear_gradient() {
    let (mut shell, mut tools) = rig();
    press(&mut shell, &mut tools, ExtControl::AngleUp);
    assert_eq!(
        shell.model.status, "Could not apply that change",
        "solid has no angle"
    );
    press(&mut shell, &mut tools, ExtControl::FillKind(1));
    let src = press_undoable(&mut shell, &mut tools, ExtControl::AngleUp);
    assert!(src.contains("y2=\"0.6294\""), "{src}");
    assert_eq!(ext(&shell).angle, Some(15.0));
    for _ in 0..5 {
        press(&mut shell, &mut tools, ExtControl::AngleUp);
    }
    assert_eq!(ext(&shell).angle, Some(90.0));
    assert_eq!(
        ext(&shell).stops,
        Some([[255, 0, 0], [255, 255, 255]]),
        "stops kept"
    );
    press(&mut shell, &mut tools, ExtControl::FillKind(2));
    press(&mut shell, &mut tools, ExtControl::AngleDown);
    assert_eq!(
        shell.model.status, "Could not apply that change",
        "radial has no angle"
    );
}

#[test]
fn presses_without_a_selection_or_ext_change_nothing_and_are_consumed() {
    let mut shell = Shell::new();
    shell.edit(ID, 10.0);
    let mut tools = ToolController::new();
    let before = shell.source();
    let cursor = shell.cursor();
    for index in EXT_FIRST..EXT_END {
        assert!(toolbar_actions::inspector_edit(
            index,
            &mut shell.model,
            &mut shell.files,
            &mut tools
        ));
        assert_eq!(shell.model.status, "Select a shape first");
    }
    assert_eq!(shell.source(), before);
    assert_eq!(shell.cursor(), cursor);
    // Unknown indices are still ignored.
    assert!(!toolbar_actions::inspector_edit(
        EXT_END,
        &mut shell.model,
        &mut shell.files,
        &mut tools
    ));
}

#[test]
fn pivot_defaults_to_the_centre_for_each_shape_kind() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100">
<ellipse id="e" cx="30" cy="40" rx="10" ry="20" fill="#fff"/>
<path id="p" d="M 0 0 L 40 0 L 40 20 Z" fill="#fff"/>
<rect id="t" x="10" y="10" width="20" height="40" transform="translate(5 6)"/></svg>"##;
    let doc = V1Document::from_source(svg).unwrap();
    let pivot = |id: &str| match ShapeExt::read(&doc, id).unwrap().transform {
        TransformView::Parts(p) => p.pivot,
        TransformView::Custom => panic!("{id}"),
    };
    assert_eq!(pivot("e"), (30.0, 40.0));
    assert_eq!(pivot("p"), (20.0, 10.0));
    assert_eq!(pivot("t"), (20.0, 30.0), "local centre, translate excluded");
    // A rotation written elsewhere about an arbitrary pivot still reads back
    // as rotate + scale, and re-applying it reproduces the matrix.
    let mut doc = doc;
    let spec = TransformSpec {
        rotate_deg: 30.0,
        pivot: (3.0, 4.0),
        scale: (2.0, 1.5),
        ..TransformSpec::default()
    };
    doc.set_transform("p", &spec).unwrap();
    let want = doc.get_transform_matrix("p").unwrap();
    let TransformView::Parts(p) = ShapeExt::read(&doc, "p").unwrap().transform else {
        panic!()
    };
    assert_eq!((p.rotation, p.scale), (30.0, (2.0, 1.5)));
    doc.set_transform(
        "p",
        &TransformSpec {
            translate: p.translate,
            rotate_deg: p.rotation,
            pivot: p.pivot,
            scale: p.scale,
            skew_x_deg: 0.0,
        },
    )
    .unwrap();
    let got = doc.get_transform_matrix("p").unwrap();
    for i in 0..6 {
        assert!((got[i] - want[i]).abs() < 1e-6, "{i}: {got:?} vs {want:?}");
    }
}

fn inspector_pixels(model: &Model) -> (Vec<u8>, u32, vexel_ui::Rect) {
    let layout = model.layout(1280, 820).unwrap();
    let frame = render(1280, 820, 1, model).expect("shell renders");
    (
        frame.pixels().to_vec(),
        frame.width(),
        layout.inspector.unwrap(),
    )
}

fn differs(
    a: &(Vec<u8>, u32, vexel_ui::Rect),
    b: &(Vec<u8>, u32, vexel_ui::Rect),
    r: vexel_ui::Rect,
) -> bool {
    (r.y..r.y + r.height).any(|y| {
        let row = (y * a.1 + r.x) as usize * 4;
        let end = row + r.width as usize * 4;
        a.0[row..end] != b.0[row..end]
    })
}

#[test]
fn rows_paint_in_their_rects_and_follow_the_selection_state() {
    let (mut shell, mut tools) = rig();
    shell.model.mode = Mode::Draw;
    let solid = inspector_pixels(&shell.model);
    let layout = shell.layout();
    // Dash preset: highlight moves, only inside the dash row.
    press(&mut shell, &mut tools, ExtControl::Dash(1));
    let dashed = inspector_pixels(&shell.model);
    let dash_row = layout.ext_row(ExtRow::Dash).unwrap();
    assert!(differs(&solid, &dashed, dash_row));
    let outside = vexel_ui::Rect {
        height: dash_row.y - 2 - layout.inspector.unwrap().y,
        ..layout.inspector.unwrap()
    };
    let first = (outside.y..outside.y + outside.height).find(|y| {
        let row = (y * solid.1 + outside.x) as usize * 4;
        let end = row + outside.width as usize * 4;
        solid.0[row..end] != dashed.0[row..end]
    });
    assert_eq!(first, None, "nothing above the dash row moved");
    // Gradient rows paint stop swatches and the angle field only for gradients.
    press(&mut shell, &mut tools, ExtControl::FillKind(1));
    let linear = inspector_pixels(&shell.model);
    for row in [ExtRow::StopFirst, ExtRow::StopLast, ExtRow::Angle] {
        assert!(
            differs(&dashed, &linear, layout.ext_row(row).unwrap()),
            "{row:?}"
        );
    }
    // Opacity and transform values repaint.
    press(&mut shell, &mut tools, ExtControl::OpacityDown);
    let faded = inspector_pixels(&shell.model);
    assert!(differs(
        &linear,
        &faded,
        layout.ext_row(ExtRow::Opacity).unwrap()
    ));
    press(&mut shell, &mut tools, ExtControl::RotateUp);
    let turned = inspector_pixels(&shell.model);
    assert!(differs(
        &faded,
        &turned,
        layout.ext_row(ExtRow::Rotate).unwrap()
    ));
}
