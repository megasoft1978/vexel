use crate::builder::room;
use crate::memory::{add, bytes};
use crate::types::{invalid, limit, stopped};
use crate::*;
impl SceneBuilder {
    pub fn append_shape(
        &mut self,
        scene: SceneId,
        input: ShapeInput,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        self.append_shape_inner(scene, input, control, None, None, 0)
    }

    /// Consume a request-scoped shape slot admitted before geometry work.
    /// All ordinary path/style validation and publication remains shared with
    /// `append_shape`; errors release the token while preserving the request.
    pub fn append_admitted_shape(
        &mut self,
        draft: &DraftToken,
        admission: &ShapeAdmission,
        input: ShapeInput,
        control: &mut dyn WorkControl,
    ) -> Result<ShapeId> {
        let (i, _) = self.shape_admission_index(draft, admission)?;
        let scene = admission.scene;
        let serial = admission.nonce;
        let geometry = admission.geometry;
        let source_offset = admission.source_offset;
        let result =
            self.append_shape_inner(scene, input, control, Some(serial), geometry, source_offset);
        let (after, position) = self.shape_admission_index(draft, admission)?;
        if i != after {
            return Err(crate::types::invalid(admission.source_offset));
        }
        self.scenes[after].shape_admissions[position].active = false;
        result
    }

    fn append_shape_inner(
        &mut self,
        scene: SceneId,
        input: ShapeInput,
        control: &mut dyn WorkControl,
        admitted_serial: Option<u64>,
        geometry: Option<crate::shape_geometry::GeometryProof>,
        source_offset: usize,
    ) -> Result<ShapeId> {
        let i = self.writable(scene)?;
        control.checkpoint().map_err(stopped)?;
        if input.path.edges.len() != input.edge_colors.len() {
            return Err(invalid(0));
        }
        crate::validate::shape_attributes(self, scene, &input.shape, control)?;
        let path_range = geometry.map_or(input.shape.source.element, |proof| proof.path_data);
        crate::validate::path(&self.source, &input.path, path_range, control)?;
        if let Some(proof) = geometry {
            crate::shape_geometry::validate_path_document(&input.path, proof.path_data, control)?;
        }
        let s = &self.scenes[i];
        let mut pending_admissions = 0usize;
        for entry in &s.shape_admissions {
            control.charge(WorkKind::TokenVisit, 1).map_err(|stop| {
                let mut error = stopped(stop);
                error.offset = source_offset;
                error
            })?;
            if entry.active {
                pending_admissions = add(pending_admissions, 1)?;
            }
        }
        let shape_growth = if admitted_serial.is_some() {
            pending_admissions
        } else {
            add(pending_admissions, 1)?
        };
        for (len, extra, cap, kind) in [
            (
                s.shapes.len(),
                shape_growth,
                self.limits.shapes,
                LimitKind::Shapes,
            ),
            (
                s.edges.len(),
                input.path.edges.len(),
                self.limits.edges,
                LimitKind::Edges,
            ),
            (
                s.path_segments.len(),
                input.path.segments.len(),
                self.limits.commands,
                LimitKind::PathCommands,
            ),
            (
                s.path_numbers.len(),
                input.path.numbers.len(),
                self.limits.numbers,
                LimitKind::PathNumbers,
            ),
        ] {
            if add(len, extra)? > cap {
                return Err(limit(kind));
            }
        }
        let pending = add(
            add(
                add(bytes(&input.path.edges)?, bytes(&input.path.subpaths)?)?,
                add(bytes(&input.path.segments)?, bytes(&input.path.numbers)?)?,
            )?,
            bytes(&input.edge_colors)?,
        )?;
        self.pending(pending)?;
        let serial = match admitted_serial {
            Some(serial) => serial,
            None => self.serial()?,
        };
        room!(self, i, shapes, shape_growth, self.limits.shapes, pending);
        room!(
            self,
            i,
            shapes_serials,
            shape_growth,
            self.limits.shapes,
            pending
        );
        room!(
            self,
            i,
            edges,
            input.path.edges.len(),
            self.limits.edges,
            pending
        );
        room!(
            self,
            i,
            edge_colors,
            input.edge_colors.len(),
            self.limits.edges,
            pending
        );
        room!(
            self,
            i,
            edge_shape_ids,
            input.path.edges.len(),
            self.limits.edges,
            pending
        );
        room!(
            self,
            i,
            path_subpaths,
            input.path.subpaths.len(),
            self.limits.commands,
            pending
        );
        room!(
            self,
            i,
            path_segments,
            input.path.segments.len(),
            self.limits.commands,
            pending
        );
        room!(
            self,
            i,
            path_numbers,
            input.path.numbers.len(),
            self.limits.numbers,
            pending
        );
        let mark = self.checkpoint_internal(scene)?;
        let id = ShapeId {
            owner: self.owner,
            scene: scene.slot,
            slot: self.scenes[i].shapes.len() as u32,
            serial,
        };
        let result = self.publish_shape(i, id, input, geometry, control);
        if result.is_err() {
            self.rollback_append(mark)?;
        }
        result.map(|()| id)
    }
    fn publish_shape(
        &mut self,
        i: usize,
        id: ShapeId,
        input: ShapeInput,
        geometry: Option<crate::shape_geometry::GeometryProof>,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        let scene = &mut self.scenes[i];
        let edge_first = scene.edges.len();
        let number_first = scene.path_numbers.len();
        let number_count = input.path.numbers.len();
        let mut shape = input.shape;
        shape.edges = TableRange {
            first: edge_first,
            count: input.path.edges.len(),
        };
        shape.subpaths = TableRange {
            first: scene.path_subpaths.len(),
            count: input.path.subpaths.len(),
        };
        for (edge, color) in input.path.edges.into_iter().zip(input.edge_colors) {
            control.charge(WorkKind::Edge, 1).map_err(stopped)?;
            scene.edges.push(edge);
            scene.edge_colors.push(color);
            scene.edge_shape_ids.push(id.slot);
        }
        for mut subpath in input.path.subpaths {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            subpath.first_edge = add(subpath.first_edge, edge_first)?;
            scene.path_subpaths.push(subpath);
        }
        for mut path in input.path.segments {
            control.charge(WorkKind::TokenVisit, 1).map_err(stopped)?;
            path.first_edge = add(path.first_edge, edge_first)?;
            path.first_number = add(path.first_number, number_first)?;
            scene
                .path_segments
                .push(ScenePathSegment { path, shape: id });
        }
        for number in input.path.numbers {
            control.charge(WorkKind::SourceRead, 1).map_err(stopped)?;
            scene.path_numbers.push(number);
        }
        control.checkpoint().map_err(stopped)?;
        scene.shapes.push(shape);
        scene.shapes_serials.push(id.serial);
        if let Some(proof) = geometry {
            scene.geometry_sources.push(crate::ShapeGeometrySource {
                shape_slot: id.slot,
                proof,
                number_range: TableRange {
                    first: number_first,
                    count: number_count,
                },
            });
        }
        Ok(())
    }
}
