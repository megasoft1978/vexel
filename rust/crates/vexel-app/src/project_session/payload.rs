//! Bounded owned copy of the opaque payload retained by a project session.

use vexel_project::{Asset, BundleInput, BundleView};

#[derive(Debug)]
pub(super) struct SessionPayload {
    manifest_json: Vec<u8>,
    timeline_json: Vec<u8>,
    assets: Vec<OwnedAsset>,
}

#[derive(Debug)]
struct OwnedAsset {
    name: Vec<u8>,
    media_type: Vec<u8>,
    data: Vec<u8>,
}

impl SessionPayload {
    pub(super) fn try_from_view_with(
        view: BundleView<'_>,
        reserve_allowed: &mut impl FnMut() -> bool,
    ) -> Result<Self, ()> {
        let manifest_json = copy_bytes(view.manifest_json, reserve_allowed)?;
        let timeline_json = copy_bytes(view.timeline_json, reserve_allowed)?;
        let mut assets = Vec::new();
        reserve(&mut assets, view.asset_count, reserve_allowed)?;
        for index in 0..view.asset_count {
            let asset = view.asset_at(index).expect("validated bundle asset");
            assets.push(OwnedAsset {
                name: copy_bytes(asset.name, reserve_allowed)?,
                media_type: copy_bytes(asset.media_type, reserve_allowed)?,
                data: copy_bytes(asset.data, reserve_allowed)?,
            });
        }
        Ok(Self {
            manifest_json,
            timeline_json,
            assets,
        })
    }

    pub(super) fn defaults(width: u32, height: u32) -> Self {
        Self {
            manifest_json: format!(
                "{{\"format\":\"vexel-project\",\"width\":{width},\"height\":{height}}}"
            )
            .into_bytes(),
            timeline_json: b"{}".to_vec(),
            assets: Vec::new(),
        }
    }

    pub(super) fn with_bundle_input<R>(
        &self,
        scene_svg: &[u8],
        write: impl FnOnce(&BundleInput<'_>) -> R,
    ) -> R {
        self.try_with_bundle_input(scene_svg, &mut || true, |input| Ok(write(input)))
            .expect("bundle asset vector allocation failed")
    }

    pub(super) fn try_with_bundle_input<R>(
        &self,
        scene_svg: &[u8],
        reserve_allowed: &mut impl FnMut() -> bool,
        write: impl FnOnce(&BundleInput<'_>) -> Result<R, ()>,
    ) -> Result<R, ()> {
        let mut assets = Vec::new();
        reserve(&mut assets, self.assets.len(), reserve_allowed)?;
        for asset in &self.assets {
            assets.push(Asset {
                name: &asset.name,
                media_type: &asset.media_type,
                data: &asset.data,
            });
        }
        let input = BundleInput {
            manifest_json: &self.manifest_json,
            scene_svg,
            timeline_json: &self.timeline_json,
            assets: &assets,
        };
        write(&input)
    }
}

fn copy_bytes(source: &[u8], reserve_allowed: &mut impl FnMut() -> bool) -> Result<Vec<u8>, ()> {
    let mut bytes = Vec::new();
    reserve(&mut bytes, source.len(), reserve_allowed)?;
    bytes.extend_from_slice(source);
    Ok(bytes)
}

fn reserve<T>(
    values: &mut Vec<T>,
    additional: usize,
    reserve_allowed: &mut impl FnMut() -> bool,
) -> Result<(), ()> {
    if additional == 0 {
        return Ok(());
    }
    if !reserve_allowed() {
        return Err(());
    }
    values.try_reserve_exact(additional).map_err(|_| ())
}
