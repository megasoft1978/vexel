use std::sync::Arc;
use vexel_spatial::{Bounds, MAX_RECORDS, Record, SpatialError, SpatialIndex};

fn bounds(x: f64, y: f64, width: f64, height: f64) -> Bounds {
    Bounds {
        x,
        y,
        width,
        height,
    }
}

fn record(id: u64, x: f64, y: f64, width: f64, height: f64) -> Record {
    Record {
        id,
        bounds: bounds(x, y, width, height),
    }
}

#[test]
fn inclusive_edges_degenerate_rectangles_duplicates_and_stable_order() {
    let records = [
        record(90, 2.0, 2.0, 0.0, 0.0),
        record(10, 0.0, 0.0, 2.0, 2.0),
        record(55, 2.0, 0.0, 0.0, 2.0),
        record(2, 20.0, 20.0, 1.0, 1.0),
    ];
    let index = SpatialIndex::new(&records).unwrap();
    let mut output = [usize::MAX; 4];
    assert_eq!(
        index
            .query(bounds(2.0, 2.0, 0.0, 0.0), &mut output)
            .unwrap(),
        3
    );
    assert_eq!(&output[..3], &[0, 1, 2]);
    assert_eq!(index.count(), 4);
}

#[test]
fn query_errors_preserve_output_and_report_required_count() {
    let index =
        SpatialIndex::new(&[record(1, 0.0, 0.0, 1.0, 1.0), record(2, 1.0, 0.0, 1.0, 1.0)]).unwrap();
    let mut short = [77; 1];
    assert_eq!(
        index.query(bounds(1.0, 0.0, 0.0, 1.0), &mut short),
        Err(SpatialError::InsufficientCapacity { required: 2 })
    );
    assert_eq!(short, [77]);
    let mut output = [44, 45];
    assert_eq!(
        index.query(bounds(f64::NAN, 0.0, 1.0, 1.0), &mut output),
        Err(SpatialError::Invalid)
    );
    assert_eq!(output, [44, 45]);
}

#[test]
fn owned_copy_survives_source_mutation_and_drop() {
    let index = {
        let mut source = vec![record(8, 5.0, 6.0, 1.0, 1.0)];
        let index = SpatialIndex::new(&source).unwrap();
        source[0] = record(9, 100.0, 100.0, 1.0, 1.0);
        index
    };
    let mut output = [usize::MAX];
    assert_eq!(index.query(bounds(5.0, 6.0, 1.0, 1.0), &mut output), Ok(1));
    assert_eq!(output, [0]);
}

#[test]
fn rejects_invalid_bounds_duplicate_ids_and_record_limit() {
    for b in [
        bounds(f64::NAN, 0.0, 1.0, 1.0),
        bounds(0.0, f64::INFINITY, 1.0, 1.0),
        bounds(0.0, 0.0, -1.0, 1.0),
        bounds(f64::MAX, 0.0, f64::MAX, 1.0),
    ] {
        assert_eq!(
            SpatialIndex::new(&[Record { id: 1, bounds: b }]).unwrap_err(),
            SpatialError::Invalid
        );
    }
    assert_eq!(
        SpatialIndex::new(&[record(1, 0.0, 0.0, 0.0, 0.0), record(1, 2.0, 2.0, 1.0, 1.0)])
            .unwrap_err(),
        SpatialError::Invalid
    );
    let too_many = vec![record(1, 0.0, 0.0, 0.0, 0.0); MAX_RECORDS + 1];
    assert_eq!(
        SpatialIndex::new(&too_many).unwrap_err(),
        SpatialError::Limit
    );
}

#[test]
fn rejects_unrepresentable_combined_index_extent() {
    let records = [
        record(1, -f64::MAX, 0.0, 0.0, 0.0),
        record(2, f64::MAX, 0.0, 0.0, 0.0),
    ];
    assert_eq!(
        SpatialIndex::new(&records).unwrap_err(),
        SpatialError::Overflow
    );
}

#[test]
fn extent_overflow_precedes_duplicate_id_rejection() {
    let records = [
        record(7, -f64::MAX, 0.0, 0.0, 0.0),
        record(7, f64::MAX, 0.0, 0.0, 0.0),
    ];
    assert_eq!(
        SpatialIndex::new(&records).unwrap_err(),
        SpatialError::Overflow
    );
}

#[test]
fn bounds_have_no_extra_geometry_cap_and_empty_index_is_valid() {
    let index = SpatialIndex::new(&[record(1, 1.0e100, -1.0e100, 1.0e90, 1.0e90)]).unwrap();
    let mut output = [usize::MAX];
    assert_eq!(
        index.query(bounds(1.0e100, -1.0e100, 0.0, 0.0), &mut output),
        Ok(1)
    );
    let empty = SpatialIndex::new(&[]).unwrap();
    assert!(empty.is_empty());
    assert_eq!(empty.query(bounds(0.0, 0.0, 0.0, 0.0), &mut output), Ok(0));
}

#[test]
fn immutable_queries_are_safe_in_parallel() {
    let index = Arc::new(
        SpatialIndex::new(&[
            record(1, -2.0, -2.0, 4.0, 4.0),
            record(2, 0.0, 0.0, 2.0, 2.0),
            record(3, 50.0, 50.0, 0.0, 0.0),
        ])
        .unwrap(),
    );
    std::thread::scope(|scope| {
        for _ in 0..8 {
            let index = Arc::clone(&index);
            scope.spawn(move || {
                for _ in 0..100 {
                    let mut output = [usize::MAX; 3];
                    assert_eq!(index.query(bounds(0.0, 0.0, 0.0, 0.0), &mut output), Ok(2));
                    assert_eq!(&output[..2], &[0, 1]);
                }
            });
        }
    });
}
