//! `scene_apply`: a whole scene (element tree, tracks, rig, duration,
//! background) validated first and applied as one atomic, undoable edit.

use super::mcp::{McpHost, McpOutcome, source, stable_ids};
use super::{AppDocument, check_room};
use crate::animation::driver::Driver;
use crate::animation::playback::LoopClock;
use crate::preview::add_extras::AddExtras;
use crate::preview::native_timeline::{NativeOp, NativeTrack};
use crate::preview::{Operation, SCENE_LIMIT};
use crate::project_session::SessionEditError;
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use vexel_v1_document::TextSpec;

/// What one element entry does.
#[derive(Clone, Debug, PartialEq)]
pub enum StepKind {
    /// Any existing add operation (shapes, groups, `use`, fragments).
    Op(Operation),
    Text {
        parent: String,
        id: Option<String>,
        spec: TextSpec,
    },
}

/// One element entry, flattened in document order; `label` names it in
/// errors (`elements[3].children[1]`) and `ids` are the ids it declares.
#[derive(Clone, Debug, PartialEq)]
pub struct Step {
    pub label: String,
    pub kind: StepKind,
    pub ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scene {
    pub steps: Vec<Step>,
    pub tracks: Option<Vec<NativeTrack>>,
    pub rig: Option<Operation>,
    /// Bones the rig declares (for the reply).
    pub rig_bones: usize,
    pub duration: Option<u32>,
    /// `#rrggbb` of the full-canvas background rect.
    pub background: Option<String>,
}

/// Most distinct error texts interned: errors travel as `&'static str`.
const MAX_INTERNED: usize = 4096;

fn leak(message: String) -> &'static str {
    static TABLE: Mutex<Option<HashMap<String, &'static str>>> = Mutex::new(None);
    let Ok(mut guard) = TABLE.lock() else {
        return "scene_apply: scene rejected; nothing changed";
    };
    let table = guard.get_or_insert_with(HashMap::new);
    if let Some(text) = table.get(&message) {
        return text;
    }
    if table.len() >= MAX_INTERNED {
        return "scene_apply: scene rejected; nothing changed (see docs/mcp.md)";
    }
    let text: &'static str = Box::leak(message.clone().into_boxed_str());
    table.insert(message, text);
    text
}

/// The inner tool errors already start with their tool name.
fn at(label: &str, inner: &str) -> &'static str {
    leak(format!("scene_apply {label}: {inner}"))
}

/// `"id":"x"` / `"elementId":"x"` of a reply.
fn reply_id(json: &str) -> Option<String> {
    let at = json.find("\"elementId\":\"")? + 13;
    let len = json[at..].find('"')?;
    Some(json[at..at + len].to_owned())
}

impl McpHost {
    pub(super) fn scene_apply<C: LoopClock>(
        &mut self,
        app: &mut AppDocument,
        scene: Scene,
        driver: &mut Driver<C>,
    ) -> Result<McpOutcome, &'static str> {
        // Everything that can be checked without changing the document.
        let source_bytes = source(app)?;
        let existing: HashSet<String> = stable_ids(&source_bytes).into_iter().collect();
        drop(source_bytes);
        let mut declared: HashSet<&str> = HashSet::new();
        for step in &scene.steps {
            for id in &step.ids {
                if existing.contains(id) || !declared.insert(id.as_str()) {
                    return Err(at(
                        &step.label,
                        &format!("id {id} is already used; ids must be new and unique"),
                    ));
                }
            }
        }
        let background = usize::from(scene.background.is_some());
        if scene.background.is_some() && existing.contains("scene-background") {
            return Err("scene_apply background: the id scene-background is already used");
        }
        let counted: usize = scene.steps.iter().map(|s| s.ids.len().max(1)).sum();
        if existing.len() + counted + background + 1 > SCENE_LIMIT {
            return Err("preview scene limit reached");
        }
        check_room(app, 0)?;
        let group = app
            .begin_group()
            .map_err(|_| "scene_apply: cannot run inside a batch")?;
        match self.scene_steps(app, scene, driver) {
            Ok(reply) => {
                app.end_group(group)
                    .map_err(|_| "project history unavailable")?;
                Ok(McpOutcome::Reply(reply))
            }
            Err(message) => {
                app.rollback_group(group)
                    .map_err(|_| "scene_apply rollback failed: project history unavailable")?;
                Err(message)
            }
        }
    }

    fn scene_steps<C: LoopClock>(
        &mut self,
        app: &mut AppDocument,
        scene: Scene,
        driver: &mut Driver<C>,
    ) -> Result<String, &'static str> {
        let (width, height) = app.size();
        let mut ids: Vec<String> = Vec::with_capacity(scene.steps.len() + 1);
        if let Some(fill) = scene.background {
            let add = Operation::AddWith {
                extras: Box::new(AddExtras {
                    id: Some("scene-background".to_owned()),
                    ..AddExtras::default()
                }),
                operation: Box::new(Operation::AddBox {
                    ellipse: false,
                    x0: 0.0,
                    y0: 0.0,
                    x1: f64::from(width),
                    y1: f64::from(height),
                    fill,
                }),
            };
            self.handle_with(app, add, driver)
                .map_err(|e| at("background", e))?;
            ids.push("scene-background".to_owned());
        }
        let count = scene.steps.len();
        for step in scene.steps {
            let label = step.label;
            let id = step.ids.first().cloned();
            match step.kind {
                StepKind::Op(op) => {
                    let reply = match self.handle_with(app, op, driver) {
                        Ok(McpOutcome::Reply(json)) => json,
                        Ok(McpOutcome::Export { .. }) => return Err(at(&label, "not allowed")),
                        Err(e) => return Err(at(&label, e)),
                    };
                    ids.push(id.or_else(|| reply_id(&reply)).unwrap_or_default());
                }
                StepKind::Text { parent, id, spec } => {
                    let made = app
                        .edit(|doc| {
                            doc.set_insert_parent(Some(parent.as_str()).filter(|p| *p != "root"))?;
                            let added = doc.add_text(&spec);
                            doc.set_insert_parent(None)?;
                            let auto = added?;
                            match &id {
                                Some(id) => doc.rename_element(&auto, id).map(|()| id.clone()),
                                None => Ok(auto),
                            }
                        })
                        .map_err(|e| match e {
                            SessionEditError::Operation(_) => at(
                                &label,
                                "text_add: parent must be \"root\" or a group; x and y finite numbers, content 1-256 characters, fontSize 1-1024, fill #rrggbb",
                            ),
                            SessionEditError::History(_) => "project history unavailable",
                        })?;
                    ids.push(made);
                }
            }
        }
        if let Some(ticks) = scene.duration {
            let op = Operation::Native(NativeOp::SetDuration {
                duration_ticks: ticks,
                ticks_per_second: None,
            });
            self.handle_with(app, op, driver)
                .map_err(|e| at("duration", e))?;
        }
        let tracks = scene.tracks.as_ref().map_or(0, Vec::len);
        if let Some(list) = scene.tracks {
            // The bulk path names the failing entry itself: `tracks[i]: ...`.
            self.handle_with(app, Operation::Native(NativeOp::TracksSet(list)), driver)
                .map_err(|e| match e.strip_prefix("native_timeline_tracks_set ") {
                    Some(rest) => leak(format!("scene_apply {rest}")),
                    None => leak(format!("scene_apply tracks: {e}")),
                })?;
        }
        let bones = scene.rig_bones;
        if let Some(rig) = scene.rig {
            self.handle_with(app, rig, driver)
                .map_err(|e| at("rig", e))?;
        }
        let list = ids
            .iter()
            .map(|id| format!("\"{id}\""))
            .collect::<Vec<_>>()
            .join(",");
        Ok(format!(
            "{{\"ok\":true,\"operation\":\"scene_apply\",\"elements\":{count},\"tracks\":{tracks},\"rigBones\":{bones},\"elementIds\":[{list}]}}"
        ))
    }
}
