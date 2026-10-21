//! Shared helpers for the `ui_canvas_*` tests (included with `#[path]`).
#![allow(dead_code)]

use vexel_app::canvas::{Bounds, StagePaint, View, paint_stage};
use vexel_app::compose::{Composed, Frame, compose_document};
use vexel_ui::{Rect, Theme, Tokens, theme_tokens};

pub const COMPOSE_V1: &[u8] = include_bytes!("../../../fixtures/v1/compose/compose-v1.svg");

pub fn tokens(dark: bool) -> Tokens {
    theme_tokens(if dark { Theme::Dark } else { Theme::Light }, false)
}

pub fn rgb(color: u32) -> [u8; 3] {
    let [r, g, b, _] = color.to_be_bytes();
    [r, g, b]
}

pub fn composed(source: &[u8], size: (u32, u32)) -> Composed {
    compose_document(source, size.0, size.1, &|| false).expect("compose")
}

/// A rendered window buffer of `window` logical points at `scale`.
pub struct Shot {
    pub px: Vec<u8>,
    pub width: u32,
    pub scale: u32,
}

impl Shot {
    pub fn at(&self, x: i64, y: i64) -> [u8; 3] {
        let i = ((y * i64::from(self.width) + x) * 4) as usize;
        [self.px[i], self.px[i + 1], self.px[i + 2]]
    }
}

pub struct Scene<'a> {
    pub window: (u32, u32),
    pub stage: Rect,
    pub scale: u32,
    pub dark: bool,
    pub view: View,
    pub artboard: (f64, f64),
    pub frame: Option<&'a Frame>,
    pub selection: Option<Bounds>,
    pub hover: Option<Bounds>,
}

/// Paints `scene` into a window buffer pre-filled with an opaque sentinel.
pub fn render(scene: &Scene<'_>) -> Shot {
    let (w, h) = (scene.window.0 * scene.scale, scene.window.1 * scene.scale);
    let mut px = vec![0u8; (w * h * 4) as usize];
    for p in px.chunks_exact_mut(4) {
        p.copy_from_slice(&[0x12, 0x34, 0x56, 0xff]);
    }
    let paint = StagePaint {
        view: scene.view,
        artboard: scene.artboard,
        frame: scene.frame,
        selection: scene.selection,
        hover: scene.hover,
        rotation: None,
    };
    paint_stage(
        &mut px,
        w,
        scene.scale,
        scene.stage,
        &tokens(scene.dark),
        Some(&paint),
    );
    Shot {
        px,
        width: w,
        scale: scene.scale,
    }
}

pub fn hash(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf2_9ce4_8422_2325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Deterministic pseudo-random sequence in `[0, 1)`.
pub struct Lcg(pub u64);

impl Lcg {
    pub fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}
