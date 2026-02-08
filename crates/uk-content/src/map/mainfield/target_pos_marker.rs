use anyhow::Context;
use itertools::Itertools;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct TargetPosMarker {
    pub rotate:         DeleteMap<char, f32>,
    pub translate:      DeleteMap<char, f32>,
    pub unique_name:    Option<String>,
}

impl TargetPosMarker {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.unique_name.clone().unwrap_or_default(),
            )
        )
        .to_string()
        .into()
    }
}

impl TryFrom<&Byml> for TargetPosMarker {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("TargetPosMarker node must be HashMap")?;
        Ok(Self {
            rotate: try_get_vecf(map.get("Rotate")
                .context("TargetPosMarker must have Rotate")?)
                .context("Invalid TargetPosMarker Rotate")?,
            translate: try_get_vecf(map.get("Translate")
                .context("TargetPosMarker must have Translate")?)
                .context("Invalid TargetPosMarker Translate")?,
            unique_name: map.get("UniqueName")
                .map(|b| b.as_string()
                    .context("TargetPosMarker UniqueName must be String")
                )
                .transpose()?
                .cloned(),
        })
    }
}

impl From<TargetPosMarker> for Byml {
    fn from(val: TargetPosMarker) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("Rotate".into(), Byml::Map(val.rotate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        map.insert("Translate".into(), Byml::Map(val.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        match &val.unique_name {
            Some(p) => map.insert("UniqueName".into(), p.into()),
            None => None,
        };
        Byml::Map(map)
    }
}

impl Mergeable for TargetPosMarker {
    fn diff(&self, other: &Self) -> Self {
        Self {
            rotate: self.rotate.diff(&other.rotate),
            translate: self.translate.diff(&other.translate),
            unique_name: if other.unique_name
                .ne(&self.unique_name) { other.unique_name.clone() } else { Default::default() },
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            rotate: self.rotate.merge(&diff.rotate),
            translate: self.translate.merge(&diff.translate),
            unique_name: diff.unique_name
                .eq(&self.unique_name)
                .then(|| self.unique_name.clone())
                .or_else(|| Some(diff.unique_name.clone()))
                .expect("UniqueName should be in at least one of these files"),
        }
    }
}
