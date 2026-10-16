//! Checked boundary conversions used by Cocoa responder adapters.
use super::types::{Modifiers, Point};
pub(crate) fn modifiers(native_flags: u64) -> Modifiers { Modifiers::from_native_bits(native_flags) }
pub(crate) fn finite_point(x: f64, y: f64) -> Option<Point> { (x.is_finite() && y.is_finite() && x.abs() <= 1.0e6 && y.abs() <= 1.0e6).then_some(Point { x, y }) }
