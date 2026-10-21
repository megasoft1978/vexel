use super::V1DocumentError;
use super::path_data::{bounds, parse, scale, validate_path_data};

fn scaled(d: &str, from: (f64, f64, f64, f64), to: (f64, f64, f64, f64)) -> String {
    // Normalise number formatting (the writer emits exponent form).
    let out = String::from_utf8(scale(d.as_bytes(), from, to).unwrap()).unwrap();
    out.split_whitespace()
        .map(|t| match t.parse::<f64>() {
            Ok(v) => format!("{v}"),
            Err(_) => t.to_owned(),
        })
        .collect::<Vec<_>>()
        .join(" ")
        + " "
}

fn bbox(d: &str) -> (f64, f64, f64, f64) {
    bounds(&parse(d.as_bytes()).unwrap()).unwrap()
}

#[test]
fn quadratic_and_smooth_commands_validate() {
    for ok in [
        "M0 330 Q90 250 180 325 Q260 235 355 315 L512 512 L0 512Z",
        "M0 0 Q1 2 3 4 T5 6",
        "M0 0 C1 2 3 4 5 6 S7 8 9 10",
        "m0 0 q1 2 3 4 t5 6 s1 2 3 4",
        "M0 0 Q1 2 3 4 5 6 7 8",
    ] {
        assert_eq!(validate_path_data(ok), Ok(()), "{ok}");
    }
    for bad in [
        "M0 0 Q1 2 3",
        "M0 0 T1",
        "M0 0 S1 2 3",
        "M0 0 q1 2 3 4 5",
        "M0 0 A1 1 0 0 0 2 2",
    ] {
        assert_eq!(
            validate_path_data(bad),
            Err(V1DocumentError::InvalidPathData),
            "{bad}"
        );
    }
}

#[test]
fn bounds_include_quadratic_and_reflected_control_points() {
    assert_eq!(bbox("M0 0 Q5 10 10 0"), (0.0, 0.0, 10.0, 10.0));
    // T reflects (5,10) about (10,0) -> (15,-10).
    assert_eq!(bbox("M0 0 Q5 10 10 0 T20 0"), (0.0, -10.0, 20.0, 10.0));
    // S after C reflects (3,4) about (5,6) -> (7,8); own control (20,-5).
    assert_eq!(bbox("M0 0 C1 2 3 4 5 6 S20 -5 9 9"), (0.0, -5.0, 20.0, 9.0));
    // Relative q.
    assert_eq!(bbox("M10 10 q5 -20 10 0"), (10.0, -10.0, 20.0, 10.0));
    // T without a preceding Q uses the current point as control.
    assert_eq!(bbox("M0 0 L4 4 T8 0"), (0.0, 0.0, 8.0, 4.0));
}

#[test]
fn scale_produces_equivalent_absolute_paths() {
    let id = (0.0, 0.0, 10.0, 10.0);
    let dbl = (0.0, 0.0, 20.0, 20.0);
    assert_eq!(scaled("M0 0 Q5 10 10 0", id, dbl), "M 0 0 Q 10 20 20 0 ");
    assert_eq!(scaled("m1 1 q1 2 3 4", id, dbl), "M 2 2 Q 4 6 10 10 ");
    // T becomes an explicit Q with the reflected control point.
    assert_eq!(
        scaled("M0 0 Q2 4 4 0 T8 0", id, dbl),
        "M 0 0 Q 4 8 8 0 Q 12 -8 16 0 "
    );
    // S becomes an explicit C; reflected control derived from previous C.
    assert_eq!(
        scaled("M0 0 C1 1 2 2 3 3 S5 5 6 6", id, dbl),
        "M 0 0 C 2 2 4 4 6 6 C 8 8 10 10 12 12 "
    );
    // S without previous cubic: first control is the current point.
    assert_eq!(scaled("M0 0 S1 1 2 2", id, dbl), "M 0 0 C 0 0 2 2 4 4 ");
    // Scaled output is valid and re-scales idempotently to the same bounds.
    let out = scaled("M0 0 Q2 4 4 0 T8 0 s1 1 2 2", id, (5.0, 5.0, 15.0, 25.0));
    assert_eq!(validate_path_data(&out), Ok(()));
}
