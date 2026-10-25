//! Clip chain emission of the scene encoder: one link per distinct
//! [`GpuClipPath`] `Arc`, one chain per distinct sequence of links, child
//! edges stored once (see `encode.rs` for the layout interpretation).

use crate::bin::spans_sample_row;
use crate::error::{GpuError, UnsupportedReason};
use crate::input::GpuClipPath;
use crate::layout::{self, FILL_EVENODD, FILL_NONZERO, GpuClip, GpuEdge, to_fixed};
use std::collections::HashMap;
use std::sync::Arc;

/// Clip links and chains of one frame, deduplicated by `Arc` identity.
pub(crate) struct ClipBuilder<'a> {
    /// Child edges (absolute canvas fixed point), appended after the tile pools.
    pub(crate) edges: &'a mut Vec<GpuEdge>,
    /// Child records of each clip link (edge starts relative to `edges`).
    pub(crate) links: HashMap<*const GpuClipPath, Vec<GpuClip>>,
    /// `(clip_start, clip_count)` of each distinct chain.
    pub(crate) chains: HashMap<Vec<*const GpuClipPath>, (u32, u32)>,
    /// Clips of the groups enclosing the current node, outermost first.
    pub(crate) groups: Vec<Arc<GpuClipPath>>,
}

impl ClipBuilder<'_> {
    /// Child records of the chain `group_clips + shape_clips`: `(start,
    /// count)` in links, `(0, 0)` when unclipped.
    pub(crate) fn chain(
        &mut self,
        out: &mut Vec<GpuClip>,
        shape_clips: &[Arc<GpuClipPath>],
    ) -> Result<(u32, u32), GpuError> {
        let links: Vec<Arc<GpuClipPath>> = self.groups.iter().chain(shape_clips).cloned().collect();
        if links.is_empty() {
            return Ok((0, 0));
        }
        let key: Vec<*const GpuClipPath> = links.iter().map(|c| Arc::as_ptr(c)).collect();
        if let Some(&hit) = self.chains.get(&key) {
            return Ok(hit);
        }
        let start = out.len() as u32;
        for link in &links {
            let ptr = Arc::as_ptr(link);
            if !self.links.contains_key(&ptr) {
                let records = self.build_link(link)?;
                self.links.insert(ptr, records);
            }
            out.extend_from_slice(&self.links[&ptr]);
        }
        if out.len() > layout::MAX_CLIP_RECORDS as usize {
            return Err(GpuError::Encode(UnsupportedReason::TooManyClips));
        }
        let hit = (start, links.len() as u32);
        self.chains.insert(key, hit);
        Ok(hit)
    }

    /// One link: the union of its children that can ever cover a sample; an
    /// empty union is one record without edges.
    fn build_link(&mut self, link: &GpuClipPath) -> Result<Vec<GpuClip>, GpuError> {
        let mut records = Vec::new();
        for child in &link.children {
            let start = self.edges.len() as u32;
            for seg in &child.edges {
                let fixed = [seg.x0, seg.y0, seg.x1, seg.y1].map(to_fixed);
                let [Some(x0), Some(y0), Some(x1), Some(y1)] = fixed else {
                    return Err(GpuError::Encode(UnsupportedReason::InvalidScene));
                };
                let edge = GpuEdge { x0, y0, x1, y1 };
                if spans_sample_row(&edge) {
                    self.edges.push(edge);
                }
            }
            let edge_count = self.edges.len() as u32 - start;
            if edge_count > 0 {
                records.push(GpuClip {
                    edge_start: start,
                    edge_count,
                    fill_rule: if child.evenodd {
                        FILL_EVENODD
                    } else {
                        FILL_NONZERO
                    },
                    union_size: 0,
                });
            }
        }
        if records.is_empty() {
            records.push(GpuClip::default());
        }
        records[0].union_size = records.len() as u32;
        Ok(records)
    }
}
