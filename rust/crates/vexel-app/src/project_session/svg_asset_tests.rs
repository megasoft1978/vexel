use super::svg_asset::{OwnedProjectAsset, SvgAssetError};
use crate::animation::native_drawing::{SVG_MEDIA_TYPE, SvgGeometry};
use vexel_project::MAX_ASSET_COUNT;

const GEOMETRY: SvgGeometry = SvgGeometry {
    width: 1.0,
    height: 1.0,
    view_box: [0.0, 0.0, 1.0, 1.0],
};
const SVG: &[u8] =
    br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"/>"#;

#[test]
fn retains_exact_validated_svg_source_bytes() {
    let source = b"\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1\" height=\"1\" viewBox=\"0 0 1 1\"/>\n";
    let asset =
        OwnedProjectAsset::import(b"cel.svg", SVG_MEDIA_TYPE, source, GEOMETRY, &[]).unwrap();
    assert_eq!(asset.name, b"cel.svg");
    assert_eq!(asset.media_type, SVG_MEDIA_TYPE);
    assert_eq!(asset.source, source);
}

#[test]
fn rejects_invalid_name_type_unsafe_svg_and_geometry() {
    assert_eq!(
        OwnedProjectAsset::import(b"../x", SVG_MEDIA_TYPE, SVG, GEOMETRY, &[]).unwrap_err(),
        SvgAssetError::InvalidName
    );
    assert_eq!(
        OwnedProjectAsset::import(b"x.svg", b"image/png", SVG, GEOMETRY, &[]).unwrap_err(),
        SvgAssetError::InvalidMediaType
    );
    assert!(matches!(OwnedProjectAsset::import(b"x.svg", SVG_MEDIA_TYPE,
        br#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1" viewBox="0 0 1 1"><script/></svg>"#,
        GEOMETRY, &[]), Err(SvgAssetError::Drawing(_))));
    let wrong = SvgGeometry {
        width: 2.0,
        ..GEOMETRY
    };
    assert!(matches!(
        OwnedProjectAsset::import(b"x.svg", SVG_MEDIA_TYPE, SVG, wrong, &[]),
        Err(SvgAssetError::Drawing(_))
    ));
}

#[test]
fn rejects_duplicate_names_and_asset_count_limit() {
    let existing = [OwnedProjectAsset {
        name: b"x.svg".to_vec(),
        media_type: SVG_MEDIA_TYPE.to_vec(),
        source: Vec::new(),
    }];
    assert_eq!(
        OwnedProjectAsset::import(b"x.svg", SVG_MEDIA_TYPE, SVG, GEOMETRY, &existing).unwrap_err(),
        SvgAssetError::DuplicateName
    );
    let full = (0..MAX_ASSET_COUNT)
        .map(|index| OwnedProjectAsset {
            name: format!("asset-{index}.svg").into_bytes(),
            media_type: SVG_MEDIA_TYPE.to_vec(),
            source: Vec::new(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        OwnedProjectAsset::import(b"new.svg", SVG_MEDIA_TYPE, SVG, GEOMETRY, &full).unwrap_err(),
        SvgAssetError::Limit
    );
}
