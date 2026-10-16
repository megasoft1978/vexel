// Exact original stroke_decode_and_render_checks inputs.
#![allow(dead_code)]
pub const DEFAULTS: &str =
    "<svg width=\"8\" height=\"8\"><line x1=\"1\" y1=\"1\" x2=\"7\" y2=\"1\"/></svg>";
pub const SOURCE: &str = "<svg width=\"16\" height=\"16\"><line x1=\"3\" y1=\"8\" x2=\"13\" y2=\"8\" stroke=\"red\" stroke-width=\"2\" stroke-linecap=\"round\" stroke-dasharray=\"3 2 1\" stroke-dashoffset=\"-1\"/></svg>";
pub const TRANSFORMED: &str = "<svg width=\"16\" height=\"16\"><g transform=\"scale(2)\"><line x1=\"1\" y1=\"1\" x2=\"6\" y2=\"1\" stroke=\"blue\" stroke-width=\"1\"/></g></svg>";
pub const ANISOTROPIC: &str = "<svg width=\"16\" height=\"16\"><g transform=\"scale(2 1)\"><line x1=\"2\" y1=\"2\" x2=\"6\" y2=\"2\" stroke=\"blue\" stroke-width=\"2\"/></g></svg>";
pub const JOINS: &str = "<svg width=\"16\" height=\"16\"><polyline points=\"2 2 8 8 14 2\" stroke=\"red\" stroke-linejoin=\"round\" stroke-miterlimit=\"1\"/></svg>";
pub const PAINTER: &str = "<svg width=\"8\" height=\"8\"><rect x=\"1\" y=\"1\" width=\"6\" height=\"6\" fill=\"blue\" stroke=\"red\" stroke-width=\"2\"/></svg>";
pub const LIMIT_WORK: &str = "<svg width=\"4096\" height=\"4096\"><line x1=\"1\" y1=\"1\" x2=\"2\" y2=\"1\" stroke=\"red\" stroke-dasharray=\"0.0000001 0.0000001\"/></svg>";
pub const INVALID: &str =
    "<svg width=\"8\" height=\"8\"><line stroke=\"red\" stroke-dasharray=\"1,,2\"/></svg>";
pub const STROKED_PATH: &str = "<svg width=\"8\" height=\"8\"><path fill=\"none\" stroke=\"blue\" stroke-width=\"1\" d=\"M1 1 L7 7\"/></svg>";
