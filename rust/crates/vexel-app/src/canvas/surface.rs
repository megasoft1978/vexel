//! A clipped device-pixel drawing surface over an RGBA8 buffer.

/// Half-open device pixel rectangle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clip {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Clip {
    pub fn intersect(self, x0: i64, y0: i64, x1: i64, y1: i64) -> Clip {
        Clip {
            x0: self.x0.max(x0),
            y0: self.y0.max(y0),
            x1: self.x1.min(x1).max(self.x0.max(x0)),
            y1: self.y1.min(y1).max(self.y0.max(y0)),
        }
    }
}

pub struct Surface<'a> {
    pub px: &'a mut [u8],
    /// Buffer width in device pixels.
    pub width: i64,
    pub clip: Clip,
    /// Device pixels per logical point.
    pub scale: i64,
}

impl Surface<'_> {
    fn index(&self, x: i64, y: i64) -> Option<usize> {
        let inside = x >= self.clip.x0 && x < self.clip.x1 && y >= self.clip.y0 && y < self.clip.y1;
        let at = ((y * self.width + x) * 4) as usize;
        (inside && x < self.width && at + 4 <= self.px.len()).then_some(at)
    }

    /// Blends `rgb` over the pixel with coverage `a` in 0..=1.
    pub fn blend(&mut self, x: i64, y: i64, rgb: [u8; 3], a: f64) {
        if a <= 0.0 {
            return;
        }
        let Some(at) = self.index(x, y) else {
            return;
        };
        let a = a.min(1.0) as f32;
        for (c, src) in self.px[at..at + 3].iter_mut().zip(rgb) {
            let d = f32::from(*c);
            *c = (d + (f32::from(src) - d) * a).round() as u8;
        }
        self.px[at + 3] = 255;
    }

    pub fn set(&mut self, x: i64, y: i64, rgb: [u8; 3]) {
        if let Some(at) = self.index(x, y) {
            self.px[at..at + 3].copy_from_slice(&rgb);
            self.px[at + 3] = 255;
        }
    }

    pub fn fill(&mut self, x0: i64, y0: i64, x1: i64, y1: i64, rgb: [u8; 3]) {
        let c = self.clip.intersect(x0, y0, x1, y1);
        for y in c.y0..c.y1 {
            for x in c.x0..c.x1 {
                self.set(x, y, rgb);
            }
        }
    }
}

/// Opaque RGB of a `0xRRGGBBAA` token.
pub fn rgb(color: u32) -> [u8; 3] {
    let [r, g, b, _] = color.to_be_bytes();
    [r, g, b]
}
