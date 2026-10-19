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
    pub(super) fn from_view(view: BundleView<'_>) -> Self {
        let assets = (0..view.asset_count)
            .map(|index| {
                let asset = view.asset_at(index).expect("validated bundle asset");
                OwnedAsset {
                    name: asset.name.to_vec(),
                    media_type: asset.media_type.to_vec(),
                    data: asset.data.to_vec(),
                }
            })
            .collect();
        Self {
            manifest_json: view.manifest_json.to_vec(),
            timeline_json: view.timeline_json.to_vec(),
            assets,
        }
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
        let assets = self
            .assets
            .iter()
            .map(|asset| Asset {
                name: &asset.name,
                media_type: &asset.media_type,
                data: &asset.data,
            })
            .collect::<Vec<_>>();
        let input = BundleInput {
            manifest_json: &self.manifest_json,
            scene_svg,
            timeline_json: &self.timeline_json,
            assets: &assets,
        };
        write(&input)
    }
}
