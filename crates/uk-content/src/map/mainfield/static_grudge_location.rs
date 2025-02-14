use anyhow::Context;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{DeleteVec, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StaticGrudgeLocation {
    pub eyeball_hash_id:    Option<u32>,
    pub translate:          DeleteVec<(char, f32)>,
}

impl TryFrom<&Byml> for StaticGrudgeLocation {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("StaticGrudgeLocation node must be HashMap")?;
        Ok(Self {
            eyeball_hash_id: map.get("EyeballHashId")
                .map(|b| b.as_u32().context("EyeballHashId must be u32"))
                .transpose()?,
            translate: map.get("Translate")
                .context("StaticGrudgeLocation must have Translate")?
                .as_map()
                .context("Invalid StaticGrudgeLocation Translate")?
                .iter()
                .enumerate()
                .map(|(i, (k, v))| {
                    match (k.chars().next(), v.as_float()) {
                        (Some(d), Ok(f)) => Ok((d, f)),
                        _ => Err(anyhow::anyhow!("Invalid StaticGrudgeLocation Translate index {i}")),
                    }
                })
                .collect::<Result<DeleteVec<_>, _>>()?,
        })
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
                .unwrap_or_default(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            eyeball_hash_id: diff.eyeball_hash_id
                .eq(&self.eyeball_hash_id)
                .then(|| self.eyeball_hash_id)
                .or_else(|| Some(diff.eyeball_hash_id))
                .unwrap(),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
