//! V1 loop playback and GIF export (WO-RUST-V1-ANIM-01).
//!
//! `frame` turns SVG source into pixels through `evaluate_frame` and
//! `vexel-render`; `playback` is the clock-injected play/pause/scrub state and
//! bounded frame queue; `export` writes the preset GIF into a granted
//! directory through `vexel-gif`.

pub mod driver;
pub mod export;
pub mod frame;
pub mod job;
pub mod keys;
pub mod native_drawing;
pub mod native_driver;
pub mod native_export;
pub mod native_frame;
pub mod native_properties;
pub mod native_stage;
pub mod playback;
pub mod present;
pub mod stage;
pub mod strip;

#[cfg(test)]
mod native_drawing_tests;
#[cfg(test)]
mod native_driver_tests;
#[cfg(test)]
mod native_export_tests;
#[cfg(test)]
mod native_export_walkthrough_tests;
#[cfg(test)]
mod native_properties_tests;
#[cfg(test)]
mod native_stage_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_driver;
#[cfg(test)]
mod tests_ui;
