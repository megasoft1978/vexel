//! Bounded, actionable argument parsing for native timeline MCP tools.

use super::{JsonDocument, JsonType, Operation, PreviewArgumentError, native_timeline::NativeOp};
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
                doc,
                args,
                tool,
                "valueType",
                1,
                32,
                "one of `scalar`, `rgba`, `transform2d` or `boolean`",
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
            value: value(doc, args, tool)?,
            name: if doc.object_get(args, b"name").is_some() {
                name("name")?
            } else {
                Vec::new()
            },
        },
        "set_property_key" => NativeOp::SetPropertyKey {
            track_id: id("trackId")?,
            tick: tick(doc, args, tool, "tick")?,
            value: value(doc, args, tool)?,
        },
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
    )
    .then(|| parse_one(doc, args, tool).map(Operation::Native))
}
