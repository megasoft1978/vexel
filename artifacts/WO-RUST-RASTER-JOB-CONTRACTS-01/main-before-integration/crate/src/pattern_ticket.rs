use crate::types::{invalid, limit, stopped};
use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatternTicket {
    owner: u64,
    serial: u64,
}
#[derive(Debug)]
struct Pending {
    ticket: PatternTicket,
    descriptor: PatternDescriptor,
    scene: SceneId,
}
/// One outstanding nested pattern at the current traversal suspension point.
/// Wrong/stale tickets and failed completions never advance that continuation.
#[derive(Debug)]
pub struct PatternTickets {
    owner: u64,
    next: u64,
    pending: Option<Pending>,
}
impl PatternTickets {
    pub fn new() -> Result<Self> {
        let owner = NEXT_OWNER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |x| x.checked_add(1))
            .map_err(|_| crate::memory::arithmetic())?;
        Ok(Self {
            owner,
            next: 1,
            pending: None,
        })
    }
    pub fn request<'a>(
        &mut self,
        child: SceneView<'a>,
        descriptor: PatternDescriptor,
        control: &mut dyn WorkControl,
    ) -> Result<PatternRequest<'a>> {
        control.checkpoint().map_err(stopped)?;
        if self.pending.is_some() {
            return Err(invalid(descriptor.error_offset));
        }
        if child.scene(child.id).is_none() {
            return Err(invalid(descriptor.error_offset));
        }
        if descriptor.width == 0
            || descriptor.height == 0
            || descriptor.width > 4096
            || descriptor.height > 4096
            || (descriptor.width as usize) * (descriptor.height as usize) > 1_048_576
        {
            return Err(limit(LimitKind::Patterns));
        }
        if !descriptor.tile_width.is_finite()
            || !descriptor.tile_height.is_finite()
            || descriptor.tile_width <= 0.0
            || descriptor.tile_height <= 0.0
            || !vexel_geometry::affine_valid(descriptor.content_transform)
        {
            return Err(invalid(descriptor.error_offset));
        }
        let ticket = PatternTicket {
            owner: self.owner,
            serial: self.next,
        };
        self.next = self
            .next
            .checked_add(1)
            .ok_or_else(crate::memory::arithmetic)?;
        self.pending = Some(Pending {
            ticket,
            descriptor,
            scene: child.id,
        });
        Ok(PatternRequest {
            ticket,
            child,
            definition: descriptor.definition,
            width: descriptor.width,
            height: descriptor.height,
            tile_width: descriptor.tile_width,
            tile_height: descriptor.tile_height,
            content_transform: descriptor.content_transform,
            error_offset: descriptor.error_offset,
        })
    }
    pub fn complete(
        &mut self,
        ticket: PatternTicket,
        tile: vexel_raster::Tile,
        control: &mut dyn WorkControl,
    ) -> Result<(SceneId, Pattern)> {
        control.checkpoint().map_err(stopped)?;
        let pending = self.pending.as_ref().ok_or_else(|| invalid(0))?;
        if pending.ticket != ticket
            || tile.width() != pending.descriptor.width
            || tile.height() != pending.descriptor.height
        {
            return Err(invalid(pending.descriptor.error_offset));
        }
        if tile.allocated_byte_count() > 16 * 1024 * 1024 {
            return Err(limit(LimitKind::Patterns));
        }
        let p = self.pending.take().ok_or_else(|| invalid(0))?;
        Ok((
            p.scene,
            Pattern {
                tile,
                definition: p.descriptor.definition,
                tile_width: p.descriptor.tile_width,
                tile_height: p.descriptor.tile_height,
                content_transform: p.descriptor.content_transform,
            },
        ))
    }
    pub fn cancel(&mut self) {
        self.pending = None;
    }
    pub fn outstanding(&self) -> bool {
        self.pending.is_some()
    }
}
