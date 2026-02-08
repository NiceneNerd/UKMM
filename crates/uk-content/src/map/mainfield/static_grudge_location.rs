use anyhow::Context;
use itertools::Itertools;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct StaticGrudgeLocation {
    pub eyeball_hash_id:    Option<u32>,
    pub translate:          DeleteMap<char, f32>,
}

impl StaticGrudgeLocation {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.eyeball_hash_id.unwrap_or_default(),
            )
        )
        .to_string()
        .into()
    }
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
            translate: try_get_vecf(map.get("Translate")
                .context("StaticGrudgeLocation must have Translate")?)
                .context("Invalid StaticGrudgeLocation Translate")?,
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
            .collect::<HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for StaticGrudgeLocation {
    fn diff(&self, other: &Self) -> Self {
        Self {
            eyeball_hash_id: if other.eyeball_hash_id
                .ne(&self.eyeball_hash_id) { other.eyeball_hash_id } else { Default::default() },
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            eyeball_hash_id: diff.eyeball_hash_id
                .eq(&self.eyeball_hash_id)
                .then_some(self.eyeball_hash_id)
                .or(Some(diff.eyeball_hash_id))
                .expect("EyeballHashId should be in at least one of these files"),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
