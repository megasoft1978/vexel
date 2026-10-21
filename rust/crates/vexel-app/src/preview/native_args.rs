//! Bounded, actionable argument parsing for native timeline MCP tools.

use super::{native_timeline::NativeOp, JsonDocument, JsonType, Operation, PreviewArgumentError};
use vexel_timeline::LoopMode;

const VALUE_LIMIT: usize = 4096;

fn sanitized(value: &[u8]) -> String {
    value
        .iter()
        .copied()
        .filter(|byte| byte.is_ascii_alphanumeric() || matches!(*byte, b'_' | b'.' | b':' | b'-'))
        .take(32)
        .map(char::from)
        .collect()
}

fn invalid(tool: &str, key: &str, form: &str, value: Option<&[u8]>) -> PreviewArgumentError {
    let echo = value.map(sanitized).filter(|s| !s.is_empty());
    PreviewArgumentError::Actionable(match echo {
        Some(value) => {
            format!("native_timeline_{tool} argument {key} `{value}` is invalid; use {form}")
        }
        None => format!("native_timeline_{tool} argument {key} is missing or invalid; use {form}"),
    })
}

fn string(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
    min: usize,
    max: usize,
    form: &str,
) -> Result<Vec<u8>, PreviewArgumentError> {
    let token = doc.object_get(args, key.as_bytes());
    let value = token.and_then(|token| doc.decoded_string(token));
    if let Some(value) = value.filter(|value| {
        value.len() >= min
            && value.len() <= max
            && std::str::from_utf8(value).is_ok()
            && token.is_some_and(|token| {
                doc.raw_token(token)
                    .is_some_and(|raw| raw.len() <= max.saturating_mul(6).saturating_add(2))
            })
    }) {
        return Ok(value);
    }
    Err(invalid(
        tool,
        key,
        form,
        token.and_then(|at| doc.decoded_string(at)).as_deref(),
    ))
}

fn tick(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
    key: &'static str,
) -> Result<u32, PreviewArgumentError> {
    doc.object_get(args, key.as_bytes())
        .filter(|token| {
            doc.token(*token)
                .is_some_and(|value| value.kind == JsonType::Number)
        })
        .and_then(|token| doc.uint(token))
        .and_then(|value| u32::try_from(value).ok())
        .ok_or_else(|| invalid(tool, key, "an integer from 0 through 4294967295", None))
}

fn value(doc: &JsonDocument<'_>, args: usize, tool: &str) -> Result<Vec<u8>, PreviewArgumentError> {
    let form = "a JSON number, true/false, or an array of numbers (at most 4096 bytes)";
    doc.object_get(args, b"value")
        .filter(|token| {
            doc.token(*token).is_some_and(|value| {
                matches!(
                    value.kind,
                    JsonType::Number | JsonType::True | JsonType::False | JsonType::Array
                )
            })
        })
        .and_then(|token| doc.raw_token(token))
        .filter(|raw| raw.len() <= VALUE_LIMIT)
        .map(<[u8]>::to_vec)
        .ok_or_else(|| invalid(tool, "value", form, None))
}

fn parse_one(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<NativeOp, PreviewArgumentError> {
    let ident = "1-128 ASCII letters, digits, `_` or `-`";
    let id = |key| string(doc, args, tool, key, 1, 128, ident);
    let name = |key| {
        string(
            doc,
            args,
            tool,
            key,
            0,
            256,
            "a UTF-8 string of at most 256 bytes",
        )
    };
    Ok(match tool {
        "add_track" => NativeOp::AddTrack {
            track_id: id("trackId")?,
            target_id: string(
                doc,
                args,
                tool,
                "targetId",
                1,
                256,
                "an existing scene element id (1-256 UTF-8 bytes), e.g. `vexel-object-1`",
            )?,
            asset_name: string(
                doc,
                args,
                tool,
                "assetName",
                1,
                128,
                "the name of an SVG asset already added with native_timeline_add_drawing_asset (1-128 ASCII letters, digits, `.`, `_` or `-`)",
            )?,
            name: if doc.object_get(args, b"name").is_some() {
                name("name")?
            } else {
                Vec::new()
            },
        },
        "add_drawing_asset" => NativeOp::AddDrawingAsset {
            asset_name: string(
                doc,
                args,
                tool,
                "assetName",
                1,
                128,
                "1-128 ASCII letters, digits, `.`, `_` or `-`",
            )?,
            svg: string(
                doc,
                args,
                tool,
                "svg",
                1,
                1_048_576,
                "a complete SVG string of 1-1048576 UTF-8 bytes whose root <svg> has width, height and viewBox equal to the document size",
            )?,
        },
        "set_cel" => NativeOp::SetCel {
            track_id: id("trackId")?,
            tick: tick(doc, args, tool, "tick")?,
            asset_name: string(
                doc,
                args,
                tool,
                "assetName",
                1,
                128,
                "the name of an SVG asset already added with native_timeline_add_drawing_asset (1-128 ASCII letters, digits, `.`, `_` or `-`)",
            )?,
            label: if doc.object_get(args, b"label").is_some() {
                string(
                    doc,
                    args,
                    tool,
                    "label",
                    0,
                    256,
                    "a UTF-8 string of at most 256 bytes",
                )?
            } else {
                Vec::new()
            },
        },
        "set_loop" => {
            let form = "one of `once`, `loop` or `ping_pong`";
            let mode = string(doc, args, tool, "mode", 1, 16, form)?;
            NativeOp::SetLoop(match mode.as_slice() {
                b"once" => LoopMode::Once,
                b"loop" => LoopMode::Repeat,
                b"ping_pong" => LoopMode::PingPong,
                _ => return Err(invalid(tool, "mode", form, Some(&mode))),
            })
        }
        "add_property_track" => NativeOp::AddPropertyTrack {
            track_id: id("trackId")?,
            target_id: string(
                doc,
                args,
                tool,
                "targetId",
                1,
                256,
                "an existing scene element id (1-256 UTF-8 bytes), e.g. `vexel-object-1`",
            )?,
            property: string(
                doc,
                args,
                tool,
                "property",
                1,
                128,
                "a property name, e.g. `opacity`",
            )?,
            value_type: string(
                doc, args, tool, "valueType", 1, 32,
                "one of `scalar`, `rgba`, `transform2d`, `trs` or `boolean`",
            )?,
            interpolation: string(
                doc,
                args,
                tool,
                "interpolation",
                1,
                32,
                "one of `step`, `linear` or `cubic`",
            )?,
            value: timeline_value(doc, args, tool)?,
            name: if doc.object_get(args, b"name").is_some() {
                name("name")?
            } else {
                Vec::new()
            },
        },
        "set_property_key" => NativeOp::SetPropertyKey {
            track_id: id("trackId")?,
            tick: tick(doc, args, tool, "tick")?,
            value: timeline_value(doc, args, tool)?,
            ease: optional_ease(doc, args, tool)?,
        },
        "remove_key" => NativeOp::RemovePropertyKey { track_id: id("trackId")?, tick: tick(doc, args, tool, "tick")? },
        "move_key" => NativeOp::MovePropertyKey { track_id: id("trackId")?, from_tick: tick(doc, args, tool, "fromTick")?, to_tick: tick(doc, args, tool, "toTick")? },
        "remove_track" => NativeOp::RemoveTrack { track_id: id("trackId")? },
        "rename_track" => NativeOp::RenameTrack { track_id: id("trackId")?, name: name("name")? },
        "duplicate_track" => NativeOp::DuplicateTrack { track_id: id("trackId")?, new_track_id: id("newTrackId")? },
        "set_interpolation" => {
            let interpolation = string(doc, args, tool, "interpolation", 1, 16, "one of `step`, `linear` or `cubic`")?;
            if ![b"step".as_slice(), b"linear", b"cubic"].contains(&interpolation.as_slice()) { return Err(invalid(tool, "interpolation", "one of `step`, `linear` or `cubic`", Some(&interpolation))); }
            NativeOp::SetTrackInterpolation { track_id: id("trackId")?, interpolation }
        },
        "reorder_track" => NativeOp::ReorderTrack { track_id: id("trackId")?, index: usize::try_from(tick(doc, args, tool, "index")?).map_err(|_| invalid(tool, "index", "a nonnegative integer", None))? },
        "remove_cel" => NativeOp::RemoveCel {
            track_id: id("trackId")?,
            tick: tick(doc, args, tool, "tick")?,
        },
        "move_item" => NativeOp::MoveItem {
            track_id: id("trackId")?,
            from_tick: tick(doc, args, tool, "fromTick")?,
            to_tick: tick(doc, args, tool, "toTick")?,
        },
        _ => {
            return Err(invalid(
                tool,
                "tool",
                "a supported native timeline operation",
                None,
            ));
        }
    })
}

pub(super) fn parse(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Option<Result<Operation, PreviewArgumentError>> {
    matches!(
        tool,
        "add_track"
            | "add_drawing_asset"
            | "set_cel"
            | "remove_cel"
            | "move_item"
            | "set_loop"
            | "add_property_track"
            | "set_property_key"
            | "remove_key"
            | "move_key"
            | "remove_track"
            | "rename_track"
            | "duplicate_track"
            | "set_interpolation"
            | "reorder_track"
    )
    .then(|| parse_one(doc, args, tool).map(Operation::Native))
}

fn optional_ease(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Option<Vec<u8>>, PreviewArgumentError> {
    let Some(token) = doc.object_get(args, b"ease") else {
        return Ok(None);
    };
    let Some(raw) = doc.decoded_string(token) else {
        return Err(invalid(tool, "ease", "step, linear, ease_in, ease_out, ease_in_out, ease_in_back, ease_out_back or bezier:x1,y1,x2,y2", None));
    };
    let names: &[&[u8]] = &[
        b"step",
        b"linear",
        b"ease_in",
        b"ease_out",
        b"ease_in_out",
        b"ease_in_back",
        b"ease_out_back",
    ];
    if names.contains(&raw.as_slice()) {
        let mut out = Vec::with_capacity(raw.len() + 2);
        out.push(b'"');
        out.extend_from_slice(&raw);
        out.push(b'"');
        return Ok(Some(out));
    }
    if let Some(points) = raw.strip_prefix(b"bezier:") {
        let parts: Vec<_> = points.split(|b| *b == b',').collect();
        if parts.len() == 4
            && parts.iter().all(|p| {
                !p.is_empty()
                    && std::str::from_utf8(p)
                        .ok()
                        .and_then(|s| s.parse::<f64>().ok())
                        .is_some_and(f64::is_finite)
            })
        {
            return Ok(Some(
                [
                    b"{\"bezier\":[".as_slice(),
                    parts[0],
                    b",",
                    parts[1],
                    b",",
                    parts[2],
                    b",",
                    parts[3],
                    b"]}",
                ]
                .concat(),
            ));
        }
    }
    Err(invalid(tool, "ease", "step, linear, ease_in, ease_out, ease_in_out, ease_in_back, ease_out_back or bezier:x1,y1,x2,y2", Some(&raw)))
}

fn timeline_value(
    doc: &JsonDocument<'_>,
    args: usize,
    tool: &str,
) -> Result<Vec<u8>, PreviewArgumentError> {
    let raw = value(doc, args, tool)?;
    let value_type = doc
        .object_get(args, b"valueType")
        .and_then(|t| doc.decoded_string(t));
    if value_type.as_deref() != Some(b"trs".as_slice()) {
        return Ok(raw);
    }
    let text = std::str::from_utf8(&raw).map_err(|_| {
        invalid(
            tool,
            "value",
            "an array of 7 or 8 finite numbers for TRS",
            None,
        )
    })?;
    let trimmed = text.trim();
    let Some(inner) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) else {
        return Err(invalid(
            tool,
            "value",
            "an array of 7 or 8 finite numbers for TRS",
            None,
        ));
    };
    let nums: Vec<&str> = inner.split(',').map(str::trim).collect();
    if !(nums.len() == 7 || nums.len() == 8)
        || nums.iter().any(|n| {
            n.parse::<f64>()
                .ok()
                .filter(|v| v.is_finite() && v.abs() <= 1e9)
                .is_none()
        })
    {
        return Err(invalid(
            tool,
            "value",
            "an array of 7 or 8 finite numbers for TRS",
            None,
        ));
    }
    let keys = [
        "tx",
        "ty",
        "rotation_deg",
        "sx",
        "sy",
        "pivot_x",
        "pivot_y",
        "skew_x_deg",
    ];
    let mut out = Vec::from(&b"{"[..]);
    for (i, (key, num)) in keys.iter().zip(nums).enumerate() {
        if i > 0 {
            out.push(b',')
        }
        out.extend_from_slice(format!("\"{key}\":{num}").as_bytes());
    }
    out.push(b'}');
    Ok(out)
}

#[cfg(test)]
mod lane_tests {
    use super::*;

    fn parsed(tool: &str, args: &str) -> NativeOp {
        let json = format!("{{{args}}}");
        let doc = JsonDocument::parse(json.as_bytes())
            .unwrap_or_else(|error| panic!("{error:?}: {json}"));
        let Operation::Native(op) = parse(&doc, doc.root(), tool).unwrap().unwrap() else {
            panic!("expected native operation")
        };
        op
    }

    #[test]
    fn parses_all_native_edit_tool_argument_shapes() {
        assert!(matches!(
            parsed("remove_key", r#""trackId":"t","tick":2"#),
            NativeOp::RemovePropertyKey { .. }
        ));
        assert!(matches!(
            parsed("move_key", r#""trackId":"t","fromTick":2,"toTick":3"#),
            NativeOp::MovePropertyKey { .. }
        ));
        assert!(matches!(
            parsed("remove_track", r#""trackId":"t""#),
            NativeOp::RemoveTrack { .. }
        ));
        assert!(matches!(
            parsed("rename_track", r#""trackId":"t","name":"N""#),
            NativeOp::RenameTrack { .. }
        ));
        assert!(matches!(
            parsed("duplicate_track", r#""trackId":"t","newTrackId":"t2""#),
            NativeOp::DuplicateTrack { .. }
        ));
        assert!(matches!(
            parsed(
                "set_interpolation",
                r#""trackId":"t","interpolation":"cubic""#
            ),
            NativeOp::SetTrackInterpolation { .. }
        ));
        assert!(matches!(
            parsed("reorder_track", r#""trackId":"t","index":0"#),
            NativeOp::ReorderTrack { .. }
        ));
    }

    #[test]
    fn parses_trs_arrays_and_ease_strings_as_timeline_json() {
        let NativeOp::AddPropertyTrack {
            value, value_type, ..
        } = parsed(
            "add_property_track",
            r#""trackId":"t","targetId":"node","property":"transform","valueType":"trs","interpolation":"linear","value":[1,2,90,1,1,3,4]"#,
        )
        else {
            panic!()
        };
        assert_eq!(value_type, b"trs");
        assert_eq!(
            value,
            br#"{"tx":1,"ty":2,"rotation_deg":90,"sx":1,"sy":1,"pivot_x":3,"pivot_y":4}"#
        );
        let NativeOp::SetPropertyKey { ease, .. } = parsed(
            "set_property_key",
            r#""trackId":"t","tick":2,"value":[0,0,180,1,1,0,0],"valueType":"trs","ease":"bezier:0.1,0.2,0.8,1""#,
        ) else {
            panic!()
        };
        assert_eq!(ease.unwrap(), br#"{"bezier":[0.1,0.2,0.8,1]}"#);
    }
}
