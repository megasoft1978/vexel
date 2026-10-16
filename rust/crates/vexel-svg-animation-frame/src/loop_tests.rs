use super::*;
use vexel_geometry::Point;
use vexel_scene::{SourceMap, Span, Stop, TableRange};

struct DuringEdges {
    accepted: usize,
    cancel_after: Option<usize>,
}
impl WorkControl for DuringEdges {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        if self.cancel_after == Some(self.accepted) {
            return Err(Stop::Canceled);
        }
        self.accepted += 1;
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
#[test]
fn edge_mapping_and_shape_bounds_poll_each_edge_keep_original_work_math() {
    let original = vec![
        Edge {
            a: Point { x: 1.0, y: 2.0 },
            b: Point { x: 3.0, y: 4.0 }
        };
        8
    ];
    let mut shape = Shape::new(SourceMap {
        element: Span { start: 0, end: 1 },
        transform: affine_identity(),
        transform_node: None,
    });
    shape.edges = TableRange {
        first: 0,
        count: original.len(),
    };
    let matrix = Affine {
        e: 5.0,
        ..affine_identity()
    };
    let mut unpublished = original.clone();
    let mut work = 0;
    let mut canceled = DuringEdges {
        accepted: 0,
        cancel_after: Some(3),
    };
    assert_eq!(
        map_shape_edges(&shape, matrix, &mut unpublished, &mut canceled, &mut work)
            .unwrap_err()
            .kind,
        ErrorKind::Canceled
    );
    assert_eq!(work, original.len());
    assert_eq!(canceled.accepted, 3);
    let mut canceled = DuringEdges {
        accepted: 0,
        cancel_after: Some(3),
    };
    let mut work = 0;
    assert_eq!(
        shape_bounds(
            &shape,
            &original,
            affine_identity(),
            &mut canceled,
            &mut work
        )
        .unwrap_err()
        .kind,
        ErrorKind::Canceled
    );
    assert_eq!(work, original.len());
    let mut success = DuringEdges {
        accepted: 0,
        cancel_after: None,
    };
    let mut mapped = original.clone();
    map_shape_edges(&shape, matrix, &mut mapped, &mut success, &mut 0).unwrap();
    assert_eq!(success.accepted, 8);
    assert_eq!(mapped[7].a.x, 6.0);
    let mut success = DuringEdges {
        accepted: 0,
        cancel_after: None,
    };
    assert_eq!(
        shape_bounds(&shape, &original, affine_identity(), &mut success, &mut 0).unwrap(),
        [1.0, 2.0, 3.0, 4.0]
    );
    assert_eq!(success.accepted, 8);
    assert_eq!(original[0].a.x, 1.0);
    let mut work = WORK_MAX - 8;
    count_work(&mut work, 8).unwrap();
    assert_eq!(work, WORK_MAX);
    assert_eq!(
        count_work(&mut work, 0).unwrap_err().kind,
        ErrorKind::Limit(LimitKind::Work)
    );
    assert_eq!(work, WORK_MAX);
}
