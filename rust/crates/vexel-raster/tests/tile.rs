use vexel_raster::{MAX_AXIS, Tile, TileError};

#[test]
fn rejects_zero_dimensions_and_axis_limit_violations() {
    for dimensions in [(0, 1), (1, 0), (MAX_AXIS + 1, 1), (1, MAX_AXIS + 1)] {
        assert_eq!(Tile::new(dimensions.0, dimensions.1), Err(TileError::InvalidDimensions));
    }
}

#[test]
fn exposes_dimensions_byte_count_and_deterministic_row_major_rgba() {
    let tile = Tile::new(2, 2).unwrap();
    assert_eq!((tile.width(), tile.height()), (2, 2));
    assert_eq!(tile.byte_count(), 16);
    assert_eq!(tile.rgba(), &[0, 0, 0, 255, 17, 5, 11, 255,
                              3, 29, 7, 255, 20, 34, 18, 255]);
}

#[test]
fn supports_mutable_pixels_and_zero_initialized_origin_channels() {
    let mut tile = Tile::new(1, 1).unwrap();
    assert_eq!(tile.rgba(), &[0, 0, 0, 255]);
    tile.rgba_mut().copy_from_slice(&[9, 8, 7, 6]);
    assert_eq!(tile.rgba(), &[9, 8, 7, 6]);
}

#[test]
fn repeated_creation_preserves_deterministic_storage() {
    assert_eq!(Tile::new(7, 5).unwrap(), Tile::new(7, 5).unwrap());
}

#[test]
fn accepts_the_axis_boundary() {
    let tile = Tile::new(MAX_AXIS, 1).unwrap();
    assert_eq!(tile.width(), MAX_AXIS);
    assert_eq!(tile.byte_count(), MAX_AXIS as usize * 4);
}
