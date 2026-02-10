use anyhow::Context;
use itertools::Itertools;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LocationPointer {
    pub location_priority:  Option<i32>,
    pub message_id:         Option<String>,
    pub pointer_type:       Option<i32>,
    pub save_flag:          Option<String>,
    pub show_level:         Option<i32>,
    pub translate:          DeleteMap<char, f32>,
}

impl LocationPointer {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.message_id.clone().unwrap_or_default()
            )
        )
        .to_string()
        .into()
    }
}

impl TryFrom<&Byml> for LocationPointer {
    type Error = anyhow::Error;

    fn try_from(value: &Byml) -> anyhow::Result<Self> {
        let map = value.as_map()
            .context("LocationPointer node must be HashMap")?;
        Ok(Self {
            location_priority: Some(map.get("LocationPriority")
                .context("LocationPointer must have LocationPriority")?
                .as_i32()
                .context("LocationPointer LocationPriority must be Int")?),
            message_id: map.get("MessageID")
                .map(|b| b.as_string()
                    .context("LocationPointer MessageID must be String")
                )
                .transpose()?
                .cloned(),
            pointer_type: Some(map.get("PointerType")
                .context("LocationPointer must have PointerType")?
                .as_i32()
                .context("LocationPointer PointerType must be Int")?),
            save_flag: map.get("SaveFlag")
                .map(|b| b.as_string()
                    .context("LocationPointer SaveFlag must be String")
                )
                .transpose()?
                .cloned(),
            show_level: Some(map.get("ShowLevel")
                .context("LocationPointer must have ShowLevel")?
                .as_i32()
                .context("LocationPointer ShowLevel must be Int")?),
            translate: try_get_vecf(map.get("Translate")
                .context("LocationPointer must have Translate")?)
                .context("Invalid LocationPointer Translate")?,
        })
    }
}

impl From<LocationPointer> for Byml {
    fn from(val: LocationPointer) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("LocationPriority".into(), val.location_priority.expect("LocationPriority should have been read on diff").into());
        match &val.message_id {
            Some(i) => map.insert("MessageID".into(), i.into()),
            None => None,
        };
        map.insert("PointerType".into(), val.pointer_type.expect("PointerType should have been read on diff").into());
        match &val.save_flag {
            Some(i) => map.insert("SaveFlag".into(), i.into()),
            None => None,
        };
        map.insert("ShowLevel".into(), val.show_level.expect("ShowLevel should have been read on diff").into());
        map.insert("Translate".into(), Byml::Map(val.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for LocationPointer {
    #[allow(clippy::obfuscated_if_else)]
    fn diff(&self, other: &Self) -> Self {
        Self {
            location_priority: other.location_priority
                .ne(&self.location_priority)
                .then_some(other.location_priority)
                .unwrap_or_default(),
            message_id: other.message_id
                .ne(&self.message_id)
                .then(|| other.message_id.clone())
                .unwrap_or_default(),
            pointer_type: other.pointer_type
                .ne(&self.pointer_type)
                .then_some(other.pointer_type)
                .unwrap_or_default(),
            save_flag: other.save_flag
                .ne(&self.save_flag)
                .then(|| other.save_flag.clone())
                .unwrap_or_default(),
            show_level: other.show_level
                .ne(&self.show_level)
                .then_some(other.show_level)
                .unwrap_or_default(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            location_priority: diff.location_priority
                .eq(&self.location_priority)
                .then_some(self.location_priority)
                .or(Some(diff.location_priority))
                .expect("LocationPriority should be present in at least one of these files"),
            message_id: diff.message_id
                .eq(&self.message_id)
                .then(|| self.message_id.clone())
                .or_else(|| Some(diff.message_id.clone()))
                .expect("MessageID should be present in at least one of these files"),
            pointer_type: diff.pointer_type
                .eq(&self.pointer_type)
                .then_some(self.pointer_type)
                .or(Some(diff.pointer_type))
                .expect("PointerType should be present in at least one of these files"),
            save_flag: diff.save_flag
                .eq(&self.save_flag)
                .then(|| self.save_flag.clone())
                .or_else(|| Some(diff.save_flag.clone()))
                .expect("SaveFlag should be present in at least one of these files"),
            show_level: diff.show_level
                .eq(&self.show_level)
                .then_some(self.show_level)
                .or(Some(diff.show_level))
                .expect("ShowLevel should be present in at least one of these files"),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
