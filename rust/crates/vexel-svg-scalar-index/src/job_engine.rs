//! Public allocation-free descriptors for the complete admitted index engine.
use crate::job_storage::{initialize, restore, save};
use crate::*;
use vexel_scene::*;

#[derive(Debug)]
pub struct ScalarIndex {
    pub scratch: JobRasterScratch,
    pub scanlines: JobScanlineState,
    pub rows: JobRowsState,
    pub fill: JobFillState,
    pub pixel_count: usize,
    pub marked_pixel_count: usize,
    backup: JobRasterScratch,
    checkpoint: JobRasterScratch,
    scene: SceneId,
    mapping: ViewMapping,
}
impl ScalarIndex {
    /// Builds the full owned index; actual clip geometry is supplied explicitly.
    /// Stroke/dash setup belongs to the caller's real stroke engine, which writes
    /// this same scratch's admitted stroke_cost/dash/local-edge buffers.
    pub fn build(
        view: SceneView<'_>,
        map: ViewMapping,
        job: &mut RasterJob,
        resources: &dyn ControlledKernelResources,
        control: &mut dyn WorkControl,
    ) -> Result<Self> {
        validate_source_length(view.source().len())?;
        job.validate_source(view.source_owner())?;
        let scratch = JobRasterScratch::new(job, control)?;
        let backup = match JobRasterScratch::new(job, control) {
            Ok(s) => s,
            Err(e) => {
                scratch.release(job)?;
                return Err(e);
            }
        };
        let checkpoint = match JobRasterScratch::new(job, control) {
            Ok(s) => s,
            Err(e) => {
                backup.release(job)?;
                scratch.release(job)?;
                return Err(e);
            }
        };
        let result = (|| {
            let pixel_count = build_bounds_job(view, map, job, &scratch, control)?;
            mark_boundaries_job(view, map, job, &scratch, control)?;
            mark_clip_boundaries_job(view, map, job, &scratch, &backup, resources, control)?;
            let marked_pixel_count = marked_pixel_count_job(
                job,
                &scratch,
                crate::util::mul(view.viewport.width as usize, view.viewport.height as usize)?,
                control,
            )?;
            let scanlines =
                create_scanlines_job(view, scratch.bounds, map, job, &scratch, control)?;
            let rows = prepare_rows_job(&scanlines, job, &scratch, &backup, control)?;
            prepare_rows_job(&scanlines, job, &scratch, &checkpoint, control)?;
            let fill = create_fill_cache_job(view, job, &scratch, &backup, control)?;
            if fill.max_edge_count != 0 {
                initialize(
                    job,
                    checkpoint.crossings,
                    fill.max_edge_count,
                    FillCrossing {
                        x: 0.0,
                        winding_delta: 0,
                        edge_index: 0,
                    },
                    control,
                )?;
                initialize(
                    job,
                    checkpoint.intervals,
                    crate::util::mul(fill.interval_stride, 5)?,
                    FillInterval::default(),
                    control,
                )?;
                initialize(
                    job,
                    checkpoint.pixel_runs,
                    view.viewport.width as usize,
                    PixelRun {
                        first_x: 0,
                        end_x: 0,
                        center_only: false,
                    },
                    control,
                )?;
            }
            Ok(Self {
                scratch,
                backup,
                checkpoint,
                scanlines,
                rows,
                fill,
                pixel_count,
                marked_pixel_count,
                scene: view.id,
                mapping: map,
            })
        })();
        if result.is_err() {
            checkpoint.release(job)?;
            backup.release(job)?;
            scratch.release(job)?;
        }
        result
    }
    pub fn build_row(
        &mut self,
        view: SceneView<'_>,
        y: u32,
        job: &mut RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.validate_view(view, self.mapping)?;
        build_row_job(
            &mut self.scanlines,
            &mut self.rows,
            view,
            y,
            job,
            &self.scratch,
            &self.backup,
            control,
        )
    }
    pub fn reset(&mut self, job: &mut RasterJob, control: &mut dyn WorkControl) -> Result<()> {
        reset_scanlines_job(
            &mut self.scanlines,
            &mut self.rows,
            job,
            &self.scratch,
            &self.backup,
            control,
        )
    }
    pub fn build_fill_row(
        &mut self,
        view: SceneView<'_>,
        shape: ShapeId,
        y: u32,
        map: ViewMapping,
        job: &mut RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<FillRowWork> {
        self.validate_view(view, map)?;
        build_fill_row_job(
            &mut self.fill,
            view,
            shape,
            y,
            map,
            &self.rows,
            job,
            &self.scratch,
            &self.backup,
            control,
        )
    }
    pub fn build_fill_runs(
        &mut self,
        view: SceneView<'_>,
        bounds: PixelBounds,
        y: u32,
        map: ViewMapping,
        job: &mut RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<usize> {
        self.validate_view(view, map)?;
        build_fill_runs_job(
            &mut self.fill,
            view,
            bounds,
            y,
            map,
            job,
            &self.scratch,
            &self.backup,
            control,
        )
    }
    pub fn scene_id(&self) -> SceneId {
        self.scene
    }
    pub fn mapping(&self) -> ViewMapping {
        self.mapping
    }
    pub(crate) fn validate_view(&self, view: SceneView<'_>, mapping: ViewMapping) -> Result<()> {
        if view.id != self.scene || mapping != self.mapping {
            return Err(crate::util::invalid(0));
        }
        Ok(())
    }
    pub fn active_shapes<'a>(&self, job: &'a RasterJob) -> Result<&'a [u32]> {
        job.slice(self.scratch.active_shapes)?
            .get(..self.rows.shape_count)
            .ok_or_else(|| crate::util::invalid(0))
    }
    pub fn active_edges<'a>(&self, job: &'a RasterJob) -> Result<&'a [u32]> {
        job.slice(self.scratch.active_edges)?
            .get(..self.rows.edge_count)
            .ok_or_else(|| crate::util::invalid(0))
    }
    pub fn release(self, job: &mut RasterJob) -> Result<()> {
        self.scratch.release(job)?;
        self.backup.release(job)?;
        self.checkpoint.release(job)
    }
    pub(crate) fn save_preflight(
        &self,
        job: &mut RasterJob,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        macro_rules! field { ($($name:ident),*) => { $(save(job,self.scratch.$name,self.checkpoint.$name,control)?;)* }; }
        field!(
            scanline_nodes,
            scanline_roots,
            shape_nodes,
            shape_offsets,
            active_edges,
            active_shapes,
            crossings,
            intervals,
            pixel_runs
        );
        Ok(())
    }
    pub(crate) fn restore_preflight(&self, job: &mut RasterJob) -> Result<()> {
        macro_rules! field { ($($name:ident),*) => { $(restore(job,self.checkpoint.$name,self.scratch.$name)?;)* }; }
        field!(
            scanline_nodes,
            scanline_roots,
            shape_nodes,
            shape_offsets,
            active_edges,
            active_shapes,
            crossings,
            intervals,
            pixel_runs
        );
        Ok(())
    }
}
