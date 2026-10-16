mod support;
use support::paint_clip_generator::{SeededRng, random_paint_clip_document};

#[test]
fn seeded_generator_preserves_original_paint_clip_fixture_matrix() {
    let mut first = SeededRng::new(0x5041_494e_545f_434c);
    let mut second = SeededRng::new(0x5041_494e_545f_434c);
    for index in 0..12 {
        let document = random_paint_clip_document(&mut first, index);
        assert_eq!(document, random_paint_clip_document(&mut second, index));
        assert!(document.starts_with("<svg width=\"40\" height=\"32\" viewBox=\"0 0 40 32\">"));
        assert!(document.contains("id=\"paint\""));
        assert!(document.contains("id=\"outer\""));
        assert!(document.contains("id=\"inner\""));
        assert!(document.contains("id=\"fractional\""));
        assert!(document.contains("fill=\"#503080\""));
        assert!(document.contains("fill=\"url(#paint)\""));
        assert!(document.contains("clip-path=\"url(#outer)\""));
        assert!(document.contains("clip-path=\"url(#inner)\""));
        let expected_gradient = if index % 2 == 0 {
            "linearGradient"
        } else {
            "radialGradient"
        };
        assert!(document.contains(&format!("<{expected_gradient} id=\"paint\"")));
        let units = if (index / 2) % 2 == 0 {
            "objectBoundingBox"
        } else {
            "userSpaceOnUse"
        };
        assert!(document.contains(&format!("gradientUnits=\"{units}\"")));
        let outer = if (index / 4) % 2 == 0 {
            "objectBoundingBox"
        } else {
            "userSpaceOnUse"
        };
        let inner = if outer == "objectBoundingBox" {
            "userSpaceOnUse"
        } else {
            "objectBoundingBox"
        };
        assert!(document.contains(&format!("id=\"outer\" clipPathUnits=\"{outer}\"")));
        assert!(document.contains(&format!("id=\"inner\" clipPathUnits=\"{inner}\"")));
        let fraction = ["22.25", "22.50", "22.75"][index % 3];
        assert!(document.contains(&format!("width=\"{fraction}\" height=\"32\"")));
    }
}
