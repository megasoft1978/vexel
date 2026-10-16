use vexel_raster::{MAX_ALLOCATED_BYTES, Tile, TileError, TileImportError};

#[test]
fn owned_import_preserves_pixels_pointer_and_spare_capacity() {
    let mut pixels = Vec::with_capacity(1024);
    pixels.extend_from_slice(&[12, 34, 56, 78]);
    let pointer = pixels.as_ptr();
    let capacity = pixels.capacity();
    let tile = Tile::from_owned_rgba(1, 1, pixels).unwrap();
    assert_eq!(tile.rgba(), &[12, 34, 56, 78]);
    assert_eq!(tile.rgba().as_ptr(), pointer);
    assert_eq!(tile.byte_count(), 4);
    assert_eq!(tile.allocated_byte_count(), capacity);
    assert!(tile.allocated_byte_count() > tile.byte_count());
}

#[test]
fn invalid_import_returns_original_allocation_unchanged() {
    for (width, height, expected) in [
        (
            0,
            1,
            TileImportError::InvalidLayout(TileError::InvalidDimensions),
        ),
        (
            1,
            2,
            TileImportError::InvalidByteCount {
                expected: 8,
                actual: 4,
            },
        ),
    ] {
        let mut pixels = Vec::with_capacity(32);
        pixels.extend_from_slice(&[1, 2, 3, 4]);
        let pointer = pixels.as_ptr();
        let capacity = pixels.capacity();
        let (error, returned) = Tile::from_owned_rgba(width, height, pixels).unwrap_err();
        assert_eq!(error, expected);
        assert_eq!(returned, [1, 2, 3, 4]);
        assert_eq!(returned.as_ptr(), pointer);
        assert_eq!(returned.capacity(), capacity);
    }
}

#[test]
fn import_bounds_reserved_capacity_and_new_tiles_report_capacity() {
    let mut pixels = Vec::with_capacity(MAX_ALLOCATED_BYTES + 1);
    pixels.extend_from_slice(&[4, 3, 2, 1]);
    let pointer = pixels.as_ptr();
    let capacity = pixels.capacity();
    let (error, returned) = Tile::from_owned_rgba(1, 1, pixels).unwrap_err();
    assert_eq!(
        error,
        TileImportError::CapacityLimit {
            maximum: MAX_ALLOCATED_BYTES,
            actual: capacity,
        }
    );
    assert_eq!(returned, [4, 3, 2, 1]);
    assert_eq!(returned.as_ptr(), pointer);
    assert_eq!(returned.capacity(), capacity);
    let tile = Tile::new(3, 5).unwrap();
    assert_eq!(tile.byte_count(), 60);
    assert!(tile.allocated_byte_count() >= tile.byte_count());
}
