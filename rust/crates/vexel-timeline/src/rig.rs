//! Skeletal rig model over existing group ids: bones with pivots, a parent
//! chain and an optional role map (head, torso, arms, legs, tail). Pure data
//! and validation; the app stores it in the document and writes bone poses as
//! `trs` keys of the bone's group, so children inherit through the group tree.

/// Most bones a rig may hold.
pub const MAX_BONES: usize = 128;
/// Largest accepted pivot coordinate magnitude.
pub const MAX_COORD: f64 = 1.0e9;
/// Longest bone id (the track id `rig_<id>` must stay within 128 bytes).
pub const MAX_BONE_ID: usize = 120;

/// What a bone is for; clips animate bones by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Head,
    Torso,
    ArmL,
    ArmR,
    LegL,
    LegR,
    Tail,
}

impl Role {
    pub const ALL: [Role; 7] = [
        Role::Head,
        Role::Torso,
        Role::ArmL,
        Role::ArmR,
        Role::LegL,
        Role::LegR,
        Role::Tail,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Role::Head => "head",
            Role::Torso => "torso",
            Role::ArmL => "arm_l",
            Role::ArmR => "arm_r",
            Role::LegL => "leg_l",
            Role::LegR => "leg_r",
            Role::Tail => "tail",
        }
    }

    pub fn parse(name: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.name() == name)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bone {
    /// The group id the bone moves.
    pub id: String,
    /// Rotation pivot in the group's own coordinate space.
    pub pivot: [f64; 2],
    /// Index of the parent bone.
    pub parent: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RigError {
    Empty,
    TooMany,
    BadId,
    Duplicate,
    UnknownParent,
    Cycle,
    BadPivot,
    UnknownRoleBone,
    DuplicateRole,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rig {
    pub bones: Vec<Bone>,
    pub roles: Vec<(Role, usize)>,
}

/// A bone id is also part of a track id, so it uses the track id alphabet.
pub fn valid_bone_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= MAX_BONE_ID
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

impl Rig {
    /// `bones` carry parent indices; `roles` map a role to a bone index.
    pub fn new(bones: Vec<Bone>, roles: Vec<(Role, usize)>) -> Result<Rig, RigError> {
        if bones.is_empty() {
            return Err(RigError::Empty);
        }
        if bones.len() > MAX_BONES {
            return Err(RigError::TooMany);
        }
        for (at, bone) in bones.iter().enumerate() {
            if !valid_bone_id(&bone.id) {
                return Err(RigError::BadId);
            }
            if bones[..at].iter().any(|b| b.id == bone.id) {
                return Err(RigError::Duplicate);
            }
            if !bone.pivot.iter().all(|v| v.is_finite() && v.abs() <= MAX_COORD) {
                return Err(RigError::BadPivot);
            }
            if bone.parent.is_some_and(|p| p >= bones.len()) {
                return Err(RigError::UnknownParent);
            }
        }
        for start in 0..bones.len() {
            let mut at = start;
            let mut steps = 0;
            while let Some(parent) = bones[at].parent {
                steps += 1;
                if parent == start || steps > bones.len() {
                    return Err(RigError::Cycle);
                }
                at = parent;
            }
        }
        for (at, (role, bone)) in roles.iter().enumerate() {
            if *bone >= bones.len() {
                return Err(RigError::UnknownRoleBone);
            }
            if roles[..at].iter().any(|(r, _)| r == role) {
                return Err(RigError::DuplicateRole);
            }
        }
        Ok(Rig { bones, roles })
    }

    pub fn index(&self, id: &str) -> Option<usize> {
        self.bones.iter().position(|b| b.id == id)
    }

    pub fn role(&self, role: Role) -> Option<usize> {
        self.roles.iter().find(|(r, _)| *r == role).map(|(_, b)| *b)
    }

    /// The id of the `trs` track that animates bone `index`.
    pub fn track_id(&self, index: usize) -> String {
        format!("rig_{}", self.bones[index].id)
    }
}

/// A bone's local pose: values of its group's `trs` transform.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalPose {
    /// Degrees about the bone pivot.
    pub rotate: f64,
    pub translate: [f64; 2],
    pub scale: [f64; 2],
}

impl LocalPose {
    pub const REST: LocalPose = LocalPose {
        rotate: 0.0,
        translate: [0.0, 0.0],
        scale: [1.0, 1.0],
    };

    /// Canonical `trs` track value text with the bone pivot.
    pub fn trs_json(&self, pivot: [f64; 2]) -> String {
        let n = |v: f64| v + 0.0; // -0 prints as 0
        format!(
            "{{\"tx\":{},\"ty\":{},\"rotation_deg\":{},\"sx\":{},\"sy\":{},\"pivot_x\":{},\"pivot_y\":{},\"skew_x_deg\":0}}",
            n(self.translate[0]),
            n(self.translate[1]),
            n(self.rotate),
            n(self.scale[0]),
            n(self.scale[1]),
            n(pivot[0]),
            n(pivot[1])
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bone(id: &str, parent: Option<usize>) -> Bone {
        Bone {
            id: id.into(),
            pivot: [1.0, 2.0],
            parent,
        }
    }

    #[test]
    fn accepts_a_chain_and_roles() {
        let rig = Rig::new(
            vec![bone("a", None), bone("b", Some(0))],
            vec![(Role::Head, 1)],
        )
        .unwrap();
        assert_eq!(rig.index("b"), Some(1));
        assert_eq!(rig.role(Role::Head), Some(1));
        assert_eq!(rig.track_id(0), "rig_a");
    }

    #[test]
    fn rejects_bad_rigs() {
        let one = || vec![bone("a", None)];
        assert_eq!(Rig::new(vec![], vec![]), Err(RigError::Empty));
        assert_eq!(
            Rig::new(vec![bone("a", Some(1)), bone("b", Some(0))], vec![]),
            Err(RigError::Cycle)
        );
        assert_eq!(Rig::new(vec![bone("a", Some(0))], vec![]), Err(RigError::Cycle));
        assert_eq!(Rig::new(vec![bone("a", Some(5))], vec![]), Err(RigError::UnknownParent));
        assert_eq!(Rig::new(vec![bone("a", None), bone("a", None)], vec![]), Err(RigError::Duplicate));
        assert_eq!(Rig::new(vec![bone("a.b", None)], vec![]), Err(RigError::BadId));
        assert_eq!(Rig::new(one(), vec![(Role::Head, 3)]), Err(RigError::UnknownRoleBone));
        assert_eq!(
            Rig::new(one(), vec![(Role::Head, 0), (Role::Head, 0)]),
            Err(RigError::DuplicateRole)
        );
        let mut far = bone("a", None);
        far.pivot = [f64::NAN, 0.0];
        assert_eq!(Rig::new(vec![far], vec![]), Err(RigError::BadPivot));
        far = bone("a", None);
        far.pivot = [2.0e9, 0.0];
        assert_eq!(Rig::new(vec![far], vec![]), Err(RigError::BadPivot));
        let many: Vec<Bone> = (0..=MAX_BONES).map(|i| bone(&format!("b{i}"), None)).collect();
        assert_eq!(Rig::new(many, vec![]), Err(RigError::TooMany));
    }

    #[test]
    fn trs_text_has_pivot() {
        let text = LocalPose { rotate: -0.0, ..LocalPose::REST }.trs_json([3.0, 4.5]);
        assert_eq!(
            text,
            "{\"tx\":0,\"ty\":0,\"rotation_deg\":0,\"sx\":1,\"sy\":1,\"pivot_x\":3,\"pivot_y\":4.5,\"skew_x_deg\":0}"
        );
    }
}
