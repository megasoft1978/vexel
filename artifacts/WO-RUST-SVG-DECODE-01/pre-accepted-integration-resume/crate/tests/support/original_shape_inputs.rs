// Exact C literals from tests/svg/shape/test_shape.c::main.
#![allow(dead_code)]
pub const ZERO_AREA_GRADIENT: &str = r###"<svg width="8" height="8"><defs><linearGradient id="g"><stop stop-color="red"/><stop offset="1" stop-color="blue"/></linearGradient></defs><rect width="0" height="8" fill="url(#g)"/><rect width="4" height="4" fill="red"/></svg>"###;
pub const DEFINITIONS: &str = r###"<svg width="8" height="8"><defs><rect width="2" height="2"/><symbol id="unused"><circle r="1"/></symbol></defs><rect x="3" width="2" height="2"/></svg>"###;
pub const SOURCE: &str =
    r###"<svg width="8" height="8"><rect x="1" y="1" width="6" height="6" fill="#f00"/></svg>"###;
pub const BAD: &str = r###"<svg width="8" height="8"><path d="M0 0 A3 3 0 2 0 4 4"/></svg>"###;
pub const MALFORMED_SMOOTH: &str = r###"<svg width="8" height="8"><path d="M0 0 S1 2 3"/></svg>"###;
pub const MALFORMED_QUAD_SMOOTH: &str =
    r###"<svg width="8" height="8"><path d="M0 0 T1"/></svg>"###;
pub const SMOOTH_OVERFLOW: &str =
    r###"<svg width="8" height="8"><path d="M999999 0 s2 2 2 2"/></svg>"###;
pub const QUAD_OVERFLOW: &str = r###"<svg width="8" height="8"><path d="M999999 0 t2 2"/></svg>"###;
pub const OVERSHOOT: &str =
    r###"<svg width="8" height="8"><path d="M0 0 C100 0 100 0 1 0"/></svg>"###;
pub const PATH_START: &str = r###"<svg width="8" height="8"><path d="M0 0 L1 1"###;
pub const PATH_MIDDLE: &str = r###""/><path d="M0 0 L1 1"###;
pub const PATH_END: &str = r###""/></svg>"###;
pub const BLANK: &str = r###"<svg width="8" height="8"/>"###;
pub const TRANSFORMED: &str = r###"<svg width="16" height="16"><g transform="translate(4 5 )"><g transform="scale(2)"><rect width="1" height="1"/></g></g><circle cx="10" cy="10" r="2"/><ellipse cx="10" cy="10" rx="2" ry="1"/><polygon points="1,1 3,1 3,3"/><polyline points="5 5 7 5 7 7"/><line x1="0" y1="0" x2="15" y2="15" fill="red"/></svg>"###;
pub const DIAGONAL_LINE: &str =
    r###"<svg width="16" height="16"><line x1="0" y1="0" x2="15" y2="15" fill="red"/></svg>"###;
pub const EXTREME_ELLIPSE: &str =
    r###"<svg width="8" height="8"><circle cx="1000000" cy="0" r="1"/></svg>"###;
pub const ECCENTRIC_ELLIPSE: &str = r###"<svg width="8" height="8"><ellipse cx="100000" cy="100000" rx="100000" ry="1000"/></svg>"###;
pub const LEADING_TRANSFORM_COMMA: &str =
    r###"<svg width="8" height="8"><rect width="1" height="1" transform=",translate(1)"/></svg>"###;
pub const REPEATED_TRANSFORM_COMMA: &str = r###"<svg width="8" height="8"><rect width="1" height="1" transform="translate(1),,scale(2)"/></svg>"###;
pub const ADJACENT_TRANSFORM_NUMBERS: &str = r###"<svg width="8" height="8"><rect width="1" height="1" transform="translate(1-2)"/></svg>"###;
pub const ADJACENT_POINT_NUMBERS: &str =
    r###"<svg width="8" height="8"><polygon points="1,1-2,2"/></svg>"###;
pub const LENGTHS: &str = r###"<svg width="96" height="64"><rect x="1em" y="1cm" width="2em" height="1pt"/><rect x="1" y="1" width="8" height="6" ry="3"/></svg>"###;
pub const ZERO_SHAPE: &str =
    r###"<svg width="8" height="8"><circle r="0"/><ellipse rx="0" ry="2"/></svg>"###;
