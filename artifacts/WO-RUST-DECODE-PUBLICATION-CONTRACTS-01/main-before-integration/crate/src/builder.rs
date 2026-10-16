use crate::memory::{add, arithmetic, bytes, reserve};
use crate::model::SceneData;
use crate::types::{invalid, limit, stopped};
use crate::*;
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

/// A scene-local append checkpoint. Rollback invalidates all marks for that scene.
/// It does not rewind independent child scenes, work charges or allocation history.
#[derive(Debug)]
pub struct BuildMark {
    pub(crate) scene: SceneId,
    pub(crate) epoch: u64,
    pub(crate) image_pixels: usize,
    pub(crate) lengths: [usize; 26],
    pub(crate) publications: usize,
}
/// Exact per-edge colors and parser-owned metadata; no synthetic color conversion.
#[derive(Debug)]
pub struct ShapeInput {
    pub shape: Shape,
    pub path: vexel_svg_path::PathDocument,
    pub edge_colors: Vec<[u8; 4]>,
}
#[derive(Debug)]
pub struct SceneBuilder {
    pub(crate) owner: u64,
    pub(crate) source: SourceOwner,
    pub(crate) generation: u64,
    pub(crate) scenes: Vec<SceneData>,
    pub(crate) animation: OwnedAnimation,
    pub(crate) ignored: [Ignored; 4],
    pub(crate) stats: MemoryStats,
    pub(crate) limits: Limits,
    pub(crate) drafts: [Option<crate::draft::ActiveDraft>; 64],
    pub(crate) draft_depth: usize,
    next_serial: u64,
    root: SceneId,
}

macro_rules! room {
    ($self:ident,$index:expr,$field:ident,$extra:expr,$cap:expr,$pending:expr) => {{
        let pending = $pending;
        let current = add($self.stats.live_bytes, pending)?;
        let available = $self
            .limits
            .reserved_bytes
            .checked_sub(current)
            .ok_or_else(|| limit(LimitKind::ReservedBytes))?;
        // Check all counters before the mutating reserve can transfer old values.
        let history_room = usize::MAX - $self.stats.total_allocated_bytes;
        let available = available.min(history_room);
        if $self.stats.allocation_count == usize::MAX {
            return Err(arithmetic());
        }
        let (new, old) = reserve(&mut $self.scenes[$index].$field, $extra, $cap, available)?;
        if new != 0 {
            $self.stats.peak_live_bytes = $self.stats.peak_live_bytes.max(add(current, new)?);
            $self.stats.total_allocated_bytes = add($self.stats.total_allocated_bytes, new)?;
            $self.stats.allocation_count += 1;
            $self.stats.live_bytes = add($self.stats.live_bytes - old, new)?;
        }
    }};
}
pub(crate) use room;

impl SceneBuilder {
    pub fn new(
        source: SourceOwner,
        animation: OwnedAnimation,
        generation: u64,
        viewport: Viewport,
        limits: Limits,
    ) -> Result<Self> {
        let limits = limits.effective();
        if !std::sync::Arc::ptr_eq(&source, animation.source_owner()) {
            return Err(invalid(0));
        }
        if source.len() > limits.source_bytes {
            return Err(limit(LimitKind::SourceBytes));
        }
        crate::validate::viewport(viewport)?;
        let live = animation.storage_bytes().ok_or_else(arithmetic)?;
        if live > limits.reserved_bytes {
            return Err(limit(LimitKind::ReservedBytes));
        }
        let owner = NEXT_OWNER
            .try_update(Ordering::Relaxed, Ordering::Relaxed, |v| v.checked_add(1))
            .map_err(|_| arithmetic())?;
        let root = SceneId {
            owner,
            slot: 0,
            serial: 1,
        };
        let mut value = Self {
            owner,
            source,
            generation,
            scenes: Vec::new(),
            animation,
            ignored: [Ignored::default(); 4],
            stats: MemoryStats {
                source_bytes: live,
                live_bytes: live,
                peak_live_bytes: live,
                total_allocated_bytes: 0,
                allocation_count: 0,
            },
            limits,
            drafts: [None; 64],
            draft_depth: 0,
            next_serial: 1,
            root,
        };
        value.stats.source_bytes = value.source.capacity();
        let actual = value.reserve_scene()?;
        value.scenes.push(SceneData::new(actual.serial, viewport));
        value.root = actual;
        Ok(value)
    }
    fn reserve_scene(&mut self) -> Result<SceneId> {
        let serial = self.serial()?;
        let available = self
            .limits
            .reserved_bytes
            .checked_sub(self.stats.live_bytes)
            .ok_or_else(|| limit(LimitKind::ReservedBytes))?;
        if self.stats.allocation_count == usize::MAX {
            return Err(arithmetic());
        }
        let (new, old) = reserve(
            &mut self.scenes,
            1,
            self.limits.scenes,
            available.min(usize::MAX - self.stats.total_allocated_bytes),
        )?;
        if new != 0 {
            self.stats.peak_live_bytes = self
                .stats
                .peak_live_bytes
                .max(add(self.stats.live_bytes, new)?);
            self.stats.live_bytes = add(self.stats.live_bytes - old, new)?;
            self.stats.total_allocated_bytes = add(self.stats.total_allocated_bytes, new)?;
            self.stats.allocation_count += 1;
        }
        Ok(SceneId {
            owner: self.owner,
            slot: self.scenes.len() as u32,
            serial,
        })
    }
    pub(crate) fn serial(&mut self) -> Result<u64> {
        let serial = self.next_serial;
        self.next_serial = self.next_serial.checked_add(1).ok_or_else(arithmetic)?;
        Ok(serial)
    }
    pub(crate) fn index(&self, id: SceneId) -> Result<usize> {
        let scene = self.scenes.get(id.index()).ok_or_else(|| invalid(0))?;
        if id.owner != self.owner || id.serial != scene.serial {
            return Err(invalid(0));
        }
        Ok(id.index())
    }
    pub(crate) fn writable(&self, id: SceneId) -> Result<usize> {
        let index = self.index(id)?;
        if self.scenes[index].sealed || self.active_draft().is_some_and(|draft| draft.scene != id) {
            return Err(invalid(0));
        }
        Ok(index)
    }
    pub fn source_owner(&self) -> &SourceOwner {
        &self.source
    }
    pub fn root_id(&self) -> SceneId {
        self.root
    }
    pub fn scene(&self, id: SceneId) -> Result<SceneView<'_>> {
        let i = self.index(id)?;
        Ok(self.scenes[i].view(
            self.owner,
            i,
            &self.scenes,
            &self.source,
            self.animation.view(),
            self.ignored,
        ))
    }
    pub fn memory_stats(&self) -> MemoryStats {
        self.stats
    }
    pub fn new_scene(&mut self, viewport: Viewport) -> Result<SceneId> {
        crate::validate::viewport(viewport)?;
        let id = self.reserve_scene()?;
        self.scenes.push(SceneData::new(id.serial, viewport));
        Ok(id)
    }
    pub fn mark(&self, scene: SceneId) -> Result<BuildMark> {
        if self.draft_depth != 0 {
            return Err(invalid(0));
        }
        self.checkpoint_internal(scene)
    }
    pub(crate) fn checkpoint_internal(&self, scene: SceneId) -> Result<BuildMark> {
        self.writable(scene)?;
        self.checkpoint_unrestricted(scene)
    }
    pub(crate) fn checkpoint_unrestricted(&self, scene: SceneId) -> Result<BuildMark> {
        let i = self.index(scene)?;
        Ok(BuildMark {
            scene,
            epoch: self.scenes[i].epoch,
            image_pixels: self.scenes[i].image_pixels,
            lengths: self.scenes[i].lengths(),
            publications: self.scenes[i].publications.len(),
        })
    }
    pub fn rollback(&mut self, mark: BuildMark) -> Result<()> {
        if self.draft_depth != 0 {
            return Err(invalid(0));
        }
        self.rollback_internal(mark)
    }
    pub(crate) fn rollback_internal(&mut self, mark: BuildMark) -> Result<()> {
        let i = self.writable(mark.scene)?;
        let scene = &self.scenes[i];
        if scene.publications.len() < mark.publications
            || scene.epoch != mark.epoch
            || scene
                .lengths()
                .iter()
                .zip(mark.lengths)
                .any(|(now, then)| *now < then)
        {
            return Err(invalid(0));
        }
        let epoch = scene.epoch.checked_add(1).ok_or_else(arithmetic)?;
        let before = scene.storage_bytes()?;
        self.scenes[i].truncate(mark.lengths);
        self.scenes[i]
            .publications
            .retain(|p| p.order < mark.publications);
        self.scenes[i].epoch = epoch;
        self.scenes[i].image_pixels = mark.image_pixels;
        let after = self.scenes[i].storage_bytes()?;
        self.stats.live_bytes = self.stats.live_bytes - (before - after);
        Ok(())
    }
    /// Seal children before linking them from masks. No mutable child can escape.
    pub fn seal(&mut self, id: SceneId, control: &mut dyn WorkControl) -> Result<()> {
        let i = self.index(id)?;
        if self.scenes[i].sealed
            || self.drafts[..self.draft_depth]
                .iter()
                .flatten()
                .any(|d| d.scene == id)
            || self
                .active_draft()
                .is_some_and(|d| id.index() < d.scene_count)
        {
            return Err(invalid(0));
        }
        crate::validate::scene(self, id, control)?;
        self.scenes[i].sealed = true;
        Ok(())
    }
    pub fn freeze(self) -> Result<SceneGraph> {
        self.freeze_controlled(&mut NoStop)
    }
    pub fn freeze_controlled(mut self, control: &mut dyn WorkControl) -> Result<SceneGraph> {
        control.checkpoint().map_err(stopped)?;
        if self.draft_depth != 0 {
            return Err(invalid(0));
        }
        for i in 0..self.scenes.len() {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            let id = SceneId {
                owner: self.owner,
                slot: i as u32,
                serial: self.scenes[i].serial,
            };
            crate::validate::scene(&self, id, control)?;
            self.scenes[i].sealed = true;
        }
        let mut measured = add(
            self.animation.storage_bytes().ok_or_else(arithmetic)?,
            bytes(&self.scenes)?,
        )?;
        for scene in &self.scenes {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            measured = add(measured, scene.storage_bytes_controlled(control)?)?;
        }
        if measured != self.stats.live_bytes || measured > self.limits.reserved_bytes {
            return Err(invalid(0));
        }
        Ok(SceneGraph {
            owner: self.owner,
            source: self.source,
            generation: self.generation,
            root: self.root,
            scenes: self.scenes,
            animation: self.animation,
            ignored: self.ignored,
            stats: self.stats,
        })
    }
    /// Record a global diagnostic in the innermost active draft, if any.
    /// Draft rollback restores both counts and first offsets; child commit
    /// retains its notices within the parent transaction.
    pub fn record_ignored(&mut self, kind: IgnoredKind, offset: usize) -> Result<()> {
        if offset > self.source.len() {
            return Err(invalid(offset));
        }
        let record = &mut self.ignored[kind as usize];
        record.count = record.count.checked_add(1).ok_or_else(arithmetic)?;
        if record.first_offset.is_none() {
            record.first_offset = Some(offset);
        }
        Ok(())
    }
    pub(crate) fn pending(&mut self, bytes: usize) -> Result<()> {
        let total = add(self.stats.live_bytes, bytes)?;
        if total > self.limits.reserved_bytes {
            return Err(limit(LimitKind::ReservedBytes));
        }
        self.stats.peak_live_bytes = self.stats.peak_live_bytes.max(total);
        Ok(())
    }
    pub(crate) fn retain(&mut self, bytes: usize) -> Result<()> {
        // All arithmetic and budget checks must precede any table publication.
        let live = add(self.stats.live_bytes, bytes)?;
        let total = add(self.stats.total_allocated_bytes, bytes)?;
        let count = self
            .stats
            .allocation_count
            .checked_add(usize::from(bytes != 0))
            .ok_or_else(arithmetic)?;
        if live > self.limits.reserved_bytes {
            return Err(limit(LimitKind::ReservedBytes));
        }
        self.stats.live_bytes = live;
        self.stats.total_allocated_bytes = total;
        self.stats.allocation_count = count;
        Ok(())
    }
}
pub(crate) struct NoStop;
impl WorkControl for NoStop {
    fn charge(&mut self, _: WorkKind, _: u64) -> std::result::Result<(), Stop> {
        Ok(())
    }
    fn checkpoint(&mut self) -> std::result::Result<(), Stop> {
        Ok(())
    }
}
