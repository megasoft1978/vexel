//! Instance source annotations stay distinct from referenced media token ranges.
use crate::{
    types::{invalid, stopped},
    *,
};

fn validate(
    element: ElementView<'_>,
    map: SourceMap,
    scene: SceneId,
    ledger: &ChargeLedger,
    builder: &SceneBuilder,
    control: &mut dyn WorkControl,
) -> Result<()> {
    control.checkpoint().map_err(stopped)?;
    let source = builder.source_owner();
    if !std::sync::Arc::ptr_eq(source, ledger.source_owner())
        || element.source().as_ptr() != source.as_slice().as_ptr()
        || element.source().len() != source.len()
    {
        return Err(invalid(map.element.start));
    }
    element.validate(control)?;
    if map.element.slice(source).is_none() || !vexel_geometry::affine_valid(map.transform) {
        return Err(invalid(map.element.start));
    }
    builder.writable(scene)?;
    let view = builder.traversal_scene(scene)?;
    if map
        .transform_node
        .is_some_and(|id| view.transform(id).is_none())
    {
        return Err(invalid(map.element.start));
    }
    control.checkpoint().map_err(stopped)?;
    Ok(())
}

impl TextDecodeRequest<'_, '_> {
    /// Call before entering a provider or publishing any shapes.
    pub fn validate_source_map(
        &self,
        builder: &SceneBuilder,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if self.end <= self.element.element.0
            || self.end > self.element.end
            || !vexel_geometry::affine_valid(self.transform)
        {
            return Err(invalid(self.source_map.element.start));
        }
        validate(
            self.element,
            self.source_map,
            self.scene,
            self.ledger,
            builder,
            control,
        )
    }
}
impl ImageDecodeRequest<'_, '_> {
    /// Call before entering a provider or publishing any shapes.
    pub fn validate_source_map(
        &self,
        builder: &SceneBuilder,
        control: &mut dyn WorkControl,
    ) -> Result<()> {
        if !vexel_geometry::affine_valid(self.transform) {
            return Err(invalid(self.source_map.element.start));
        }
        validate(
            self.element,
            self.source_map,
            self.scene,
            self.ledger,
            builder,
            control,
        )
    }
}
