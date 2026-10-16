use std::sync::atomic::AtomicBool;
use vexel_base::{BudgetControl, Span};
use vexel_geometry::{Point, affine_identity};
use vexel_scene::*;
use vexel_svg_definitions::{apply_group, create_rect};

#[allow(dead_code)]
mod common;
use common::*;

struct CancelOnThirdCheckpoint(usize);
impl vexel_base::WorkControl for CancelOnThirdCheckpoint {
    fn charge(
        &mut self,
        _: vexel_base::WorkKind,
        _: u64,
    ) -> std::result::Result<(), vexel_base::Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), vexel_base::Stop> {
        self.0 += 1;
        if self.0 == 3 {
            Err(vexel_base::Stop::Canceled)
        } else {
            Ok(())
        }
    }
}

#[test]
fn caller_rolls_back_late_group_apply_cancel_and_restores_draft() {
    let mut fixture = fixture(SVG);
    let baseline_shape = *fixture
        .builder
        .scene(fixture.scene)
        .unwrap()
        .shape(fixture.owner)
        .unwrap();
    let baseline_stats = fixture.builder.memory_stats();
    let baseline_clips = fixture.builder.scene(fixture.scene).unwrap().clips.len();
    let baseline_lengths = {
        let view = fixture.builder.scene(fixture.scene).unwrap();
        [
            view.edges.len(),
            view.edge_colors.len(),
            view.edge_shape_ids.len(),
            view.shapes.len(),
            view.path_subpaths.len(),
            view.path_segments.len(),
            view.path_numbers.len(),
            view.transforms.len(),
            view.images.len(),
            view.groups.len(),
            view.filters.len(),
            view.filter_primitives.len(),
            view.masks.len(),
            view.patterns.len(),
            view.fill_paints.len(),
            view.stroke_paints.len(),
            view.dash_values.len(),
            view.stops.len(),
            view.clips.len(),
        ]
    };
    let token = fixture.builder.begin_draft(fixture.scene).unwrap();
    let cancel = AtomicBool::new(false);
    let mut setup = BudgetControl::new(&cancel, u64::MAX);
    let path_span = attribute(
        fixture.source.as_slice(),
        fixture.document.tokens(),
        &fixture.document.tokens()[fixture.owner_element.0],
        b"d",
    )
    .unwrap();
    let path = vexel_svg_path::parse_path(
        fixture.source.as_slice(),
        Span {
            start: path_span.offset,
            end: path_span.offset + path_span.length,
        },
        affine_identity(),
        vexel_svg_path::ParseOptions::default(),
    )
    .unwrap();
    let mut shape = Shape::new(baseline_shape.source);
    shape.edges.count = path.edges.len();
    shape.subpaths.count = path.subpaths.len();
    let draft_shape = fixture
        .builder
        .append_shape(
            fixture.scene,
            ShapeInput {
                edge_colors: vec![[0, 0, 0, 255]; path.edges.len()],
                shape,
                path,
            },
            &mut setup,
        )
        .unwrap();
    let clip = create_rect(
        [
            Point { x: 0., y: 0. },
            Point { x: 4., y: 0. },
            Point { x: 4., y: 4. },
            Point { x: 0., y: 4. },
        ],
        affine_identity(),
        None,
        &mut fixture.builder,
        fixture.scene,
        &mut setup,
    )
    .unwrap();
    let mut cancel = CancelOnThirdCheckpoint(0);
    let error = apply_group(
        &mut fixture.builder,
        &token,
        fixture.scene,
        &[draft_shape, draft_shape],
        clip,
        &mut cancel,
    )
    .unwrap_err();
    assert_eq!(
        cancel.0, 3,
        "late cancellation occurs on the second shape after one finalization"
    );
    assert_eq!(error.kind, ErrorKind::Canceled);
    assert_eq!(
        fixture
            .builder
            .scene(fixture.scene)
            .unwrap()
            .shape(draft_shape)
            .unwrap()
            .clip,
        Some(clip)
    );
    assert_eq!(
        fixture.builder.scene(fixture.scene).unwrap().clips.len(),
        baseline_clips + 1
    );
    fixture.builder.rollback_draft(token).unwrap();
    let restored = fixture.builder.scene(fixture.scene).unwrap();
    assert_eq!(restored.shape(fixture.owner), Some(&baseline_shape));
    assert_eq!(restored.clips.len(), baseline_clips);
    assert_eq!(
        [
            restored.edges.len(),
            restored.edge_colors.len(),
            restored.edge_shape_ids.len(),
            restored.shapes.len(),
            restored.path_subpaths.len(),
            restored.path_segments.len(),
            restored.path_numbers.len(),
            restored.transforms.len(),
            restored.images.len(),
            restored.groups.len(),
            restored.filters.len(),
            restored.filter_primitives.len(),
            restored.masks.len(),
            restored.patterns.len(),
            restored.fill_paints.len(),
            restored.stroke_paints.len(),
            restored.dash_values.len(),
            restored.stops.len(),
            restored.clips.len(),
        ],
        baseline_lengths,
        "draft rollback restores every logical scene table"
    );
    assert!(fixture.builder.memory_stats().live_bytes >= baseline_stats.live_bytes);
}
