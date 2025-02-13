use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{DeleteVec, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StaticGrudgeLocation {
    pub eyeball_hash_id:    Option<u32>,
    pub translate:          DeleteVec<(char, f32)>,
}

impl From<&Byml> for StaticGrudgeLocation {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("StaticGrudgeLocation node must be HashMap");
        Self {
            eyeball_hash_id: map.get("EyeballHashId")
                .map(|b| b.as_u32().expect("EyeballHashId must be u32")),
            translate: map.get("Translate")
                .expect("StaticGrudgeLocation must have Translate")
                .as_map()
                .expect("Invalid StaticGrudgeLocation Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
        }
    }
}

impl From<StaticGrudgeLocation> for Byml {
    fn from(value: StaticGrudgeLocation) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        match value.eyeball_hash_id {
            Some(u) => map.insert("EyeballHashId".into(), u.into()),
            None => None,
        };
        map.insert("Translate".into(), Byml::Map(value.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for StaticGrudgeLocation {
    fn diff(&self, other: &Self) -> Self {
        Self {
            eyeball_hash_id: other.eyeball_hash_id
                .ne(&self.eyeball_hash_id)
                .then(|| other.eyeball_hash_id)
                .unwrap(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            eyeball_hash_id: diff.eyeball_hash_id
                .or(self.eyeball_hash_id),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
