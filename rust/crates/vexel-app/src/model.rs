use vexel_ui::Rect;

const INSPECTOR_MIN_CONTENT_HEIGHT: u32 = 414;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Draw,
    Animate,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tool {
    Select,
    Pen,
    Rectangle,
    Ellipse,
    Line,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    DrawMode,
    AnimateMode,
    Tool(Tool),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub top: Rect,
    pub draw_mode: Rect,
    pub animate_mode: Rect,
    pub rail: Rect,
    pub stage: Rect,
    pub inspector: Option<Rect>,
    pub timeline: Option<Rect>,
    pub activity: Rect,
    pub status: Rect,
    pub tools: [Rect; 5],
}
impl Layout {
    pub fn new(width: u32, height: u32, mode: Mode) -> Option<Self> {
        if width < 440 || height < 300 || width > 4096 || height > 4096 {
            return None;
        }
        let top_h = 48.min(height / 6);
        let status_h = 28.min(height / 8);
        let activity_h = 70.min(height / 5);
        let content_y = top_h;
        let content_h = height.saturating_sub(top_h + status_h + activity_h);
        if content_h < 140 {
            return None;
        }
        let rail_w = 54.min(width / 8);
        let inspector = (width >= 900 && content_h >= INSPECTOR_MIN_CONTENT_HEIGHT).then(|| {
            let w = (width / 5).clamp(208, 252);
            Rect {
                x: width - w,
                y: content_y,
                width: w,
                height: content_h,
            }
        });
        let stage_x = rail_w;
        let stage_right = inspector.map_or(width, |r| r.x);
        let stage_w = stage_right.saturating_sub(stage_x);
        if stage_w < 260 {
            return None;
        }
        let timeline_h = if mode == Mode::Animate {
            106.min(content_h / 3)
        } else {
            0
        };
        let stage = Rect {
            x: stage_x,
            y: content_y,
            width: stage_w,
            height: content_h - timeline_h,
        };
        let timeline = (timeline_h > 0).then_some(Rect {
            x: stage_x,
            y: content_y + content_h - timeline_h,
            width: stage_w,
            height: timeline_h,
        });
        let tool_h = 42.min(content_h.saturating_sub(24) / 5);
        let tool_gap = if tool_h < 42 { 2 } else { 5 };
        let tools = std::array::from_fn(|i| Rect {
            x: 5,
            y: content_y + 6 + i as u32 * (tool_h + tool_gap),
            width: rail_w.saturating_sub(10),
            height: tool_h,
        });
        let mode_w = 86;
        let mode_y = 9;
        let draw_mode = Rect {
            x: width / 2 - mode_w,
            y: mode_y,
            width: mode_w,
            height: 30,
        };
        let animate_mode = Rect {
            x: width / 2,
            y: mode_y,
            width: mode_w,
            height: 30,
        };
        Some(Self {
            width,
            height,
            top: Rect {
                x: 0,
                y: 0,
                width,
                height: top_h,
            },
            draw_mode,
            animate_mode,
            rail: Rect {
                x: 0,
                y: content_y,
                width: rail_w,
                height: content_h,
            },
            stage,
            inspector,
            timeline,
            activity: Rect {
                x: 0,
                y: height - status_h - activity_h,
                width,
                height: activity_h,
            },
            status: Rect {
                x: 0,
                y: height - status_h,
                width,
                height: status_h,
            },
            tools,
        })
    }
    pub fn hit_test(&self, point: (u32, u32)) -> Option<Target> {
        if contains(self.draw_mode, point) {
            return Some(Target::DrawMode);
        }
        if contains(self.animate_mode, point) {
            return Some(Target::AnimateMode);
        }
        for (index, rect) in self.tools.iter().enumerate() {
            if contains(*rect, point) {
                return Some(Target::Tool(
                    [
                        Tool::Select,
                        Tool::Pen,
                        Tool::Rectangle,
                        Tool::Ellipse,
                        Tool::Line,
                    ][index],
                ));
            }
        }
        None
    }
}
fn contains(r: Rect, p: (u32, u32)) -> bool {
    p.0 >= r.x
        && p.1 >= r.y
        && p.0 < r.x.saturating_add(r.width)
        && p.1 < r.y.saturating_add(r.height)
}

pub fn checked_pixel_geometry(points: (u32, u32), scale: u32) -> Option<(u32, u32)> {
    if !(1..=3).contains(&scale) {
        return None;
    }
    let width = points.0.checked_mul(scale)?;
    let height = points.1.checked_mul(scale)?;
    if width == 0 || height == 0 || width > 4096 || height > 4096 {
        return None;
    }
    let bytes = (width as usize)
        .checked_mul(height as usize)?
        .checked_mul(4)?;
    (bytes <= 64 * 1024 * 1024).then_some((width, height))
}

#[derive(Debug)]
pub struct Model {
    pub mode: Mode,
    pub dark: bool,
    pub dirty: bool,
    pub status: &'static str,
}
impl Model {
    pub fn new(dark: bool) -> Self {
        Self {
            mode: Mode::Draw,
            dark,
            dirty: true,
            status: "No document open",
        }
    }
    pub fn select_mode(&mut self, mode: Mode) -> bool {
        if self.mode == mode {
            return false;
        }
        self.mode = mode;
        self.dirty = true;
        true
    }
    pub fn key(&mut self, code: u16) -> bool {
        match code {
            2 => self.select_mode(Mode::Draw),
            0 => self.select_mode(Mode::Animate),
            _ => false,
        }
    }
    pub fn pointer(&mut self, layout: &Layout, point: (u32, u32)) -> bool {
        match layout.hit_test(point) {
            Some(Target::DrawMode) => self.select_mode(Mode::Draw),
            Some(Target::AnimateMode) => self.select_mode(Mode::Animate),
            Some(Target::Tool(_)) | None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_workbench_keeps_stage_dominant_and_timeline_docked() {
        let draw = Layout::new(1280, 820, Mode::Draw).unwrap();
        assert!(draw.inspector.is_some());
        assert!(draw.timeline.is_none());
        assert!(draw.stage.width > 700 && draw.stage.height > 500);
        let animate = Layout::new(1280, 820, Mode::Animate).unwrap();
        assert!(animate.timeline.is_some());
        assert!(animate.stage.width > 700);
    }
    #[test]
    fn compact_bounds_hide_inspector_and_keep_stage_valid() {
        let l = Layout::new(520, 420, Mode::Draw).unwrap();
        assert!(l.inspector.is_none());
        assert!(l.stage.width >= 400);
        assert!(Layout::new(439, 420, Mode::Draw).is_none());
        assert!(Layout::new(520, 299, Mode::Draw).is_none());
    }
    #[test]
    fn inspector_is_omitted_until_its_fixed_content_fits() {
        for (width, height) in [(900, 300), (900, 420), (900, 500), (1024, 400)] {
            let layout = Layout::new(width, height, Mode::Draw).unwrap();
            assert!(
                layout.inspector.is_none(),
                "{width}x{height} must hide the inspector"
            );
        }
        let compact = Layout::new(900, 520, Mode::Draw).unwrap();
        assert!(compact.inspector.is_none());
        let layout = Layout::new(900, 600, Mode::Draw).unwrap();
        let inspector = layout
            .inspector
            .expect("full inspector content fits at 520 points");
        assert!(inspector.y + inspector.height <= 520);
        assert!(layout.stage.x + layout.stage.width <= inspector.x);
        let animate = Layout::new(900, 600, Mode::Animate).unwrap();
        assert!(animate.inspector.is_some());
        assert!(animate.stage.y + animate.stage.height <= animate.timeline.unwrap().y);
    }
    #[test]
    fn compact_painted_control_geometry_stays_inside_panels_in_both_modes() {
        for mode in [Mode::Draw, Mode::Animate] {
            let compact = Layout::new(440, 300, mode).unwrap();
            for tool in compact.tools {
                assert!(tool.y >= compact.rail.y);
                assert!(tool.y + tool.height <= compact.rail.y + compact.rail.height);
            }
            if let Some(timeline) = compact.timeline {
                assert!(timeline.y + timeline.height <= compact.activity.y);
            }
            let short_wide = Layout::new(900, 520, mode).unwrap();
            assert!(short_wide.inspector.is_none());
            if let Some(timeline) = short_wide.timeline {
                assert!(short_wide.stage.y + short_wide.stage.height <= timeline.y);
                assert!(timeline.y + timeline.height <= short_wide.activity.y);
            }
        }
    }
    #[test]
    fn mode_selection_works_by_keyboard_pointer_and_is_idempotent() {
        let l = Layout::new(1280, 820, Mode::Draw).unwrap();
        let mut model = Model::new(true);
        assert!(model.key(0));
        assert_eq!(model.mode, Mode::Animate);
        model.dirty = false;
        assert!(!model.key(0));
        assert!(!model.dirty);
        assert!(model.pointer(&l, (l.draw_mode.x + 2, l.draw_mode.y + 2)));
        assert_eq!(model.mode, Mode::Draw);
    }
    #[test]
    fn rejects_unsupported_backing_scales_and_oversized_frames() {
        assert_eq!(checked_pixel_geometry((1280, 820), 2), Some((2560, 1640)));
        assert_eq!(checked_pixel_geometry((4097, 1), 1), None);
        assert_eq!(checked_pixel_geometry((1400, 820), 3), None);
        assert_eq!(checked_pixel_geometry((4096, 4096), 1), Some((4096, 4096)));
        assert_eq!(checked_pixel_geometry((100, 100), 2), Some((200, 200)));
        assert_eq!(checked_pixel_geometry((100, 100), 4), None);
    }
    #[test]
    fn pointer_hit_testing_is_deterministic_and_ignores_unavailable_actions() {
        let l = Layout::new(1280, 820, Mode::Draw).unwrap();
        assert_eq!(
            l.hit_test((l.tools[2].x + 3, l.tools[2].y + 3)),
            Some(Target::Tool(Tool::Rectangle))
        );
        let mut model = Model::new(true);
        assert!(!model.pointer(&l, (l.tools[2].x + 3, l.tools[2].y + 3)));
        assert_eq!(l.hit_test((0, 0)), None);
        assert_eq!(l.hit_test((l.stage.x + 20, l.stage.y + 20)), None);
        assert_eq!(
            l.hit_test((l.animate_mode.x, l.animate_mode.y)),
            Some(Target::AnimateMode)
        );
    }
    #[test]
    fn line_tool_is_the_fifth_rail_slot_and_fits_in_every_layout() {
        for mode in [Mode::Draw, Mode::Animate] {
            for (w, h) in [(440, 300), (900, 520), (1280, 820)] {
                let l = Layout::new(w, h, mode).unwrap();
                let line = l.tools[4];
                assert_eq!(
                    l.hit_test((line.x + 2, line.y + 2)),
                    Some(Target::Tool(Tool::Line))
                );
                for pair in l.tools.windows(2) {
                    assert!(
                        pair[0].y + pair[0].height <= pair[1].y,
                        "rail slots must not overlap"
                    );
                }
                assert!(line.y + line.height <= l.rail.y + l.rail.height);
            }
        }
    }
}
