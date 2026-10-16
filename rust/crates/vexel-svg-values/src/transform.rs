use crate::AttributeList;
use crate::number::{NumberError, Reader};
use vexel_base::Span;
use vexel_geometry::{Affine, affine_compose, affine_identity, affine_valid};

const DEG_TO_RAD: f64 = 0.017453292519943295;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransformError {
    pub offset: usize,
}

pub fn affine_inverse(matrix: Affine) -> Option<Affine> {
    if !affine_valid(matrix) {
        return None;
    }
    let determinant = matrix.a * matrix.d - matrix.b * matrix.c;
    if !determinant.is_finite() || determinant.abs() < 1e-15 {
        return None;
    }
    let reciprocal = 1.0 / determinant;
    let mut result = Affine {
        a: matrix.d * reciprocal,
        b: -matrix.b * reciprocal,
        c: -matrix.c * reciprocal,
        d: matrix.a * reciprocal,
        e: 0.0,
        f: 0.0,
    };
    result.e = -(result.a * matrix.e + result.c * matrix.f);
    result.f = -(result.b * matrix.e + result.d * matrix.f);
    affine_valid(result).then_some(result)
}

pub fn parse_transform(source: &[u8], span: Span) -> Result<(Affine, usize), TransformError> {
    parse_transform_with_limits(source, span, crate::ValueLimits::default())
}

pub fn parse_transform_with_limits(
    source: &[u8],
    span: Span,
    limits: crate::ValueLimits,
) -> Result<(Affine, usize), TransformError> {
    if source.len() > limits.effective().source_bytes {
        return Err(TransformError { offset: 0 });
    }
    let bytes = span
        .slice(source)
        .ok_or(TransformError { offset: span.start })?;
    let mut reader = Reader::new(bytes, span.start);
    let mut result = affine_identity();
    let mut operations = 0usize;
    loop {
        reader.skip_space();
        if reader.index == bytes.len() {
            return Ok((result, operations));
        }
        let name_start = reader.index;
        while matches!(bytes.get(reader.index), Some(b'a'..=b'z' | b'A'..=b'Z')) {
            reader.index += 1;
        }
        if reader.index == name_start || bytes.get(reader.index) != Some(&b'(') {
            return Err(TransformError {
                offset: reader.absolute(),
            });
        }
        let name = &bytes[name_start..reader.index];
        reader.index += 1;
        let count = match name {
            b"matrix" => 6,
            b"translate" | b"scale" => 2,
            b"rotate" => 3,
            b"skewX" | b"skewY" => 1,
            _ => {
                return Err(TransformError {
                    offset: span.start + name_start,
                });
            }
        };
        let mut values = [0.0; 6];
        reader.skip_space();
        if matches!(name, b"translate" | b"scale") {
            values[0] = reader.number().map_err(number_error)?;
            let before = reader.index;
            reader.skip_space();
            if reader.index < bytes.len() && bytes[reader.index] != b')' {
                reader.index = before;
                if !reader.separator() {
                    return Err(TransformError {
                        offset: reader.absolute(),
                    });
                }
                values[1] = reader.number().map_err(number_error)?;
            } else {
                values[1] = if name == b"scale" { values[0] } else { 0.0 };
            }
        } else if name == b"rotate" {
            values[0] = reader.number().map_err(number_error)?;
            let before = reader.index;
            reader.skip_space();
            if reader.index < bytes.len() && bytes[reader.index] != b')' {
                reader.index = before;
                if !reader.separator() {
                    return Err(TransformError {
                        offset: reader.absolute(),
                    });
                }
                values[1] = reader.number().map_err(number_error)?;
                if !reader.separator() {
                    return Err(TransformError {
                        offset: reader.absolute(),
                    });
                }
                values[2] = reader.number().map_err(number_error)?;
            }
        } else {
            for index in 0..count {
                if index != 0 && !reader.separator() {
                    return Err(TransformError {
                        offset: reader.absolute(),
                    });
                }
                values[index] = reader.number().map_err(number_error)?;
            }
        }
        reader.skip_space();
        if bytes.get(reader.index) != Some(&b')') {
            return Err(TransformError {
                offset: reader.absolute(),
            });
        }
        reader.index += 1;

        let operation = match name {
            b"matrix" => Affine {
                a: values[0],
                b: values[1],
                c: values[2],
                d: values[3],
                e: values[4],
                f: values[5],
            },
            b"translate" => Affine {
                e: values[0],
                f: values[1],
                ..affine_identity()
            },
            b"scale" => Affine {
                a: values[0],
                d: values[1],
                ..affine_identity()
            },
            b"skewX" | b"skewY" => {
                let tangent = (values[0] * DEG_TO_RAD).tan();
                if !tangent.is_finite() || tangent.abs() > 1.0e6 {
                    return Err(TransformError {
                        offset: span.start + name_start,
                    });
                }
                if name == b"skewX" {
                    Affine {
                        c: tangent,
                        ..affine_identity()
                    }
                } else {
                    Affine {
                        b: tangent,
                        ..affine_identity()
                    }
                }
            }
            b"rotate" => {
                let angle = values[0] * DEG_TO_RAD;
                let (sine, cosine) = angle.sin_cos();
                let mut rotation = Affine {
                    a: cosine,
                    b: sine,
                    c: -sine,
                    d: cosine,
                    e: 0.0,
                    f: 0.0,
                };
                if count == 3 {
                    rotation.e = values[1] - cosine * values[1] + sine * values[2];
                    rotation.f = values[2] - sine * values[1] - cosine * values[2];
                }
                rotation
            }
            _ => {
                return Err(TransformError {
                    offset: span.start + name_start,
                });
            }
        };
        operations += 1;
        if operations > 64 || !affine_valid(operation) {
            return Err(TransformError {
                offset: span.start + name_start,
            });
        }
        result = affine_compose(result, operation).ok_or(TransformError {
            offset: span.start + name_start,
        })?;
        let after_close = reader.index;
        reader.skip_space();
        let had_space = reader.index != after_close;
        if bytes.get(reader.index) == Some(&b',') {
            reader.index += 1;
            reader.skip_space();
            if reader.index == bytes.len() || bytes[reader.index] == b',' {
                return Err(TransformError {
                    offset: reader.absolute(),
                });
            }
        } else if reader.index < bytes.len() && !had_space {
            return Err(TransformError {
                offset: reader.absolute(),
            });
        }
    }
}

/// Parse an element's optional `transform` and compose it with its parent.
pub fn parse_transform_node(
    attributes: AttributeList<'_>,
    parent: Affine,
) -> Result<(Affine, usize), TransformError> {
    let Some(span) = attributes.get(b"transform") else {
        return Ok((parent, 0));
    };
    let (local, operations) =
        parse_transform_with_limits(attributes.source(), span, attributes.limits())
            .map_err(|_| TransformError { offset: span.start })?;
    let composed = affine_compose(parent, local).ok_or(TransformError {
        offset: attributes.element_name().start,
    })?;
    Ok((composed, operations))
}

fn number_error(error: NumberError) -> TransformError {
    TransformError {
        offset: error.offset,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vexel_geometry::{Point, affine_map_point};

    fn parse(source: &[u8]) -> Result<(Affine, usize), TransformError> {
        parse_transform(
            source,
            Span {
                start: 0,
                end: source.len(),
            },
        )
    }

    #[test]
    fn inverse_round_trip_limits_and_singularity() {
        let matrix = Affine {
            a: 2.0,
            b: 0.5,
            c: -0.25,
            d: 3.0,
            e: 8.0,
            f: -4.0,
        };
        let inverse = affine_inverse(matrix).unwrap();
        let point = Point { x: 12.0, y: 7.0 };
        let mapped = affine_map_point(matrix, point).unwrap();
        let restored = affine_map_point(inverse, mapped).unwrap();
        assert!((restored.x - point.x).abs() < 1e-10);
        assert!((restored.y - point.y).abs() < 1e-10);
        assert!(
            affine_inverse(Affine {
                a: 1.0,
                b: 1.0,
                c: 1.0,
                d: 1.0 + 1e-16,
                e: 0.0,
                f: 0.0
            })
            .is_none()
        );
        assert!(
            affine_inverse(Affine {
                e: 1e6,
                ..affine_identity()
            })
            .is_some()
        );
        assert!(
            affine_inverse(Affine {
                e: 1e6_f64.next_up(),
                ..affine_identity()
            })
            .is_none()
        );
    }

    #[test]
    fn transform_operations_follow_c_matrix_order_and_arity_defaults() {
        let (matrix, count) = parse(b"translate(10,20) scale(2) rotate(90)").unwrap();
        assert_eq!(count, 3);
        let point = affine_map_point(matrix, Point { x: 1.0, y: 0.0 }).unwrap();
        assert!((point.x - 10.0).abs() < 1e-12 && (point.y - 22.0).abs() < 1e-12);
        let (scale, _) = parse(b"scale(3)").unwrap();
        assert_eq!((scale.a, scale.d), (3.0, 3.0));
        let (translate, _) = parse(b"translate(3)").unwrap();
        assert_eq!((translate.e, translate.f), (3.0, 0.0));
        let (matrix, _) = parse(b"matrix(1 2 3 4 5 6)").unwrap();
        assert_eq!(
            matrix,
            Affine {
                a: 1.0,
                b: 2.0,
                c: 3.0,
                d: 4.0,
                e: 5.0,
                f: 6.0
            }
        );
        let (centered, _) = parse(b"rotate(90 4 5)").unwrap();
        let center = affine_map_point(centered, Point { x: 4.0, y: 5.0 }).unwrap();
        assert!((center.x - 4.0).abs() < 1e-12 && (center.y - 5.0).abs() < 1e-12);
        let (skew, _) = parse(b"skewX(45) skewY(-45)").unwrap();
        assert!((skew.c - 1.0).abs() < 1e-12 && (skew.b + 1.0).abs() < 1e-12);
        assert!(parse(b"translate(1-2)").is_err());
        assert!(parse(b"translate(1),,scale(2)").is_err());
        assert!(parse(b"rotate(1 2)").is_err());
    }

    #[test]
    fn transform_count_numeric_limits_and_error_offsets() {
        let source = b"  perspective(2)";
        let error = parse_transform(
            source,
            Span {
                start: 0,
                end: source.len(),
            },
        )
        .unwrap_err();
        assert_eq!(error.offset, 2);
        let too_many = b"translate(0) ".repeat(65);
        assert!(parse(&too_many).is_err());
        assert!(parse(b"matrix(1e999 0 0 1 0 0)").is_err());
    }

    #[test]
    fn transform_node_defaults_and_composes_parent_with_original_source_offsets() {
        use crate::{Attribute, AttributeList};
        let source = b"g transform translate(2 3)";
        let attribute = Attribute {
            name: Span { start: 2, end: 11 },
            value: Span { start: 12, end: 26 },
            prefix: None,
        };
        let attributes = [attribute];
        let list = AttributeList::new(source, Span { start: 0, end: 1 }, &attributes).unwrap();
        let parent = Affine {
            e: 10.0,
            f: 20.0,
            ..affine_identity()
        };
        let (composed, count) = parse_transform_node(list, parent).unwrap();
        assert_eq!(count, 1);
        assert_eq!((composed.e, composed.f), (12.0, 23.0));
        let missing = AttributeList::new(source, Span { start: 0, end: 1 }, &[]).unwrap();
        assert_eq!(parse_transform_node(missing, parent), Ok((parent, 0)));
        let bad_source = b"g transform translate(x)";
        let bad = Attribute {
            name: Span { start: 2, end: 11 },
            value: Span { start: 12, end: 24 },
            prefix: None,
        };
        let bad_attributes = [bad];
        let bad_list =
            AttributeList::new(bad_source, Span { start: 0, end: 1 }, &bad_attributes).unwrap();
        assert_eq!(
            parse_transform_node(bad_list, parent).unwrap_err().offset,
            12
        );

        let overflow_source = b"g transform scale(2)";
        let overflow_attribute = Attribute {
            name: Span { start: 2, end: 11 },
            value: Span { start: 12, end: 20 },
            prefix: None,
        };
        let overflow_attributes = [overflow_attribute];
        let overflow_list = AttributeList::new(
            overflow_source,
            Span { start: 0, end: 1 },
            &overflow_attributes,
        )
        .unwrap();
        let large_parent = Affine {
            a: 1.0e12,
            ..affine_identity()
        };
        assert_eq!(
            parse_transform_node(overflow_list, large_parent)
                .unwrap_err()
                .offset,
            0
        );
    }

    #[test]
    fn transform_policy_is_used_by_raw_and_attribute_entry_points() {
        let payload = b"g transform translate(2 3)";
        let mut source = vec![b' '; crate::MAX_SOURCE_BYTES + 1];
        source.extend_from_slice(payload);
        let name_start = source.len() - payload.len() + 2;
        let value_start = source.len() - b"translate(2 3)".len();
        let attrs = [crate::Attribute {
            name: Span {
                start: name_start,
                end: name_start + 9,
            },
            value: Span {
                start: value_start,
                end: source.len(),
            },
            prefix: None,
        }];
        let element = Span {
            start: source.len() - payload.len(),
            end: source.len() - payload.len() + 1,
        };
        let limits = crate::ValueLimits {
            source_bytes: source.len(),
            attributes: 1,
        };
        let list = crate::AttributeList::new_with_limits(&source, element, &attrs, limits).unwrap();
        let parent = affine_identity();
        let (transformed, count) = parse_transform_node(list, parent).unwrap();
        assert_eq!(count, 1);
        assert_eq!((transformed.e, transformed.f), (2.0, 3.0));
        assert_eq!(
            parse_transform_with_limits(&source, attrs[0].value, limits)
                .unwrap()
                .1,
            1
        );
        assert_eq!(
            parse_transform(&source, attrs[0].value).unwrap_err().offset,
            0
        );
    }
}
