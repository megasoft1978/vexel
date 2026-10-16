//! Bounded keyboard command map and owned accessibility snapshots.
use crate::tree::{MAX_COMMANDS, MAX_NODES, MAX_TEXT_TOTAL_BYTES, Tree, TreeError};

pub const MOD_SHIFT: u32 = 1;
pub const MOD_COMMAND: u32 = 2;
pub const MOD_OPTION: u32 = 4;
pub const MOD_CONTROL: u32 = 8;
const MOD_MASK: u32 = MOD_SHIFT | MOD_COMMAND | MOD_OPTION | MOD_CONTROL;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommandBinding {
    pub command_id: u32,
    pub key: u32,
    pub modifiers: u32,
    pub enabled: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CommandRegistry {
    bindings: Vec<CommandBinding>,
}
impl CommandRegistry {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn len(&self) -> usize {
        self.bindings.len()
    }
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
    pub fn register(
        &mut self,
        command_id: u32,
        key: u32,
        modifiers: u32,
        enabled: bool,
    ) -> Result<(), TreeError> {
        if command_id == 0 || key == 0 || modifiers & !MOD_MASK != 0 {
            return Err(TreeError::InvalidInput);
        }
        if self.bindings.len() >= MAX_COMMANDS {
            return Err(TreeError::Capacity);
        }
        if self
            .bindings
            .iter()
            .any(|b| b.command_id == command_id || (b.key == key && b.modifiers == modifiers))
        {
            return Err(TreeError::DuplicateId);
        }
        self.bindings
            .try_reserve(1)
            .map_err(|_| TreeError::Allocation)?;
        self.bindings.push(CommandBinding {
            command_id,
            key,
            modifiers,
            enabled,
        });
        Ok(())
    }
    pub(crate) fn lookup(&self, key: u32, modifiers: u32) -> Option<u32> {
        self.bindings
            .iter()
            .find(|b| b.enabled && b.key == key && b.modifiers == modifiers)
            .map(|b| b.command_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticsNode {
    pub id: u32,
    pub parent_id: u32,
    pub role: crate::Role,
    pub state: u32,
    pub command_id: u32,
    pub label_preview: String,
    pub value_preview: String,
    pub label: String,
    pub value: String,
    pub frame: crate::Rect,
}
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SemanticsSnapshot {
    pub nodes: Vec<SemanticsNode>,
    pub text_bytes_used: usize,
}
impl SemanticsSnapshot {
    pub(crate) fn from_tree(tree: &Tree) -> Result<Self, TreeError> {
        if tree.nodes.len() > MAX_NODES || tree.text_bytes_used > MAX_TEXT_TOTAL_BYTES {
            return Err(TreeError::InvalidInput);
        }
        let mut out = Self::default();
        out.nodes
            .try_reserve_exact(tree.nodes.len())
            .map_err(|_| TreeError::Allocation)?;
        for source in &tree.nodes {
            let bytes = source
                .spec
                .label
                .len()
                .checked_add(source.spec.value.len())
                .and_then(|n| out.text_bytes_used.checked_add(n))
                .filter(|n| *n <= MAX_TEXT_TOTAL_BYTES)
                .ok_or(TreeError::TextLimit)?;
            let mut label = String::new();
            label
                .try_reserve_exact(source.spec.label.len())
                .map_err(|_| TreeError::Allocation)?;
            label.push_str(&source.spec.label);
            let mut value = String::new();
            value
                .try_reserve_exact(source.spec.value.len())
                .map_err(|_| TreeError::Allocation)?;
            value.push_str(&source.spec.value);
            let mut lp = String::new();
            lp.try_reserve_exact(source.label_preview.len())
                .map_err(|_| TreeError::Allocation)?;
            lp.push_str(&source.label_preview);
            let mut vp = String::new();
            vp.try_reserve_exact(source.value_preview.len())
                .map_err(|_| TreeError::Allocation)?;
            vp.push_str(&source.value_preview);
            out.nodes.push(SemanticsNode {
                id: source.spec.id,
                parent_id: source.spec.parent_id,
                role: source.spec.role,
                state: source.spec.state,
                command_id: source.spec.command_id,
                label_preview: lp,
                value_preview: vp,
                label,
                value,
                frame: source.bounds,
            });
            out.text_bytes_used = bytes;
        }
        Ok(out)
    }
    pub fn text<'a>(&'a self, node: &'a SemanticsNode, value: bool) -> Option<&'a str> {
        if !self.nodes.iter().any(|n| std::ptr::eq(n, node)) {
            return None;
        }
        Some(if value {
            node.value.as_str()
        } else {
            node.label.as_str()
        })
    }
}
