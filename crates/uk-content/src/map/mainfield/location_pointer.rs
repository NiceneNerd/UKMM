use anyhow::Context;
use itertools::Itertools;
use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{parsers::try_get_vecf, DeleteMap, HashMap}};
use crate::util::DeleteValue;

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LocationPointer {
    pub location_priority:  Option<i32>,
    pub message_id:         DeleteValue<String>,
    pub pointer_type:       Option<i32>,
    pub save_flag:          DeleteValue<String>,
    pub show_level:         Option<i32>,
    pub translate:          DeleteMap<char, f32>,
}

impl LocationPointer {
    pub fn id(&self) -> String {
        roead::aamp::hash_name(
            &format!(
                "{}{}",
                self.translate.values().map(|v| (v * 100000.0f32).to_string()).join(""),
                self.message_id.value().unwrap_or_default()
            )
        )
        .to_string()
        .into()
    }

    pub fn is_complete(&self) -> bool {
        // Some of these are optional
        self.location_priority.is_some() &&
            //self.message_id.is_some() &&
            self.pointer_type.is_some() &&
            //self.save_flag.is_some() &&
            self.show_level.is_some() &&
            self.translate.iter().all(|(c, _)| *c == 'X' || *c == 'Y' || *c == 'Z')
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
                .cloned()
                .into(),
            pointer_type: Some(map.get("PointerType")
                .context("LocationPointer must have PointerType")?
                .as_i32()
                .context("LocationPointer PointerType must be Int")?),
            save_flag: map.get("SaveFlag")
                .map(|b| b.as_string()
                    .context("LocationPointer SaveFlag must be String")
                )
                .transpose()?
                .cloned()
                .into(),
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
        match val.message_id.value() {
            Some(i) => map.insert("MessageID".into(), i.into()),
            None => None,
        };
        map.insert("PointerType".into(), val.pointer_type.expect("PointerType should have been read on diff").into());
        match val.save_flag.value() {
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
            message_id: self.message_id.diff(&other.message_id),
            pointer_type: other.pointer_type
                .ne(&self.pointer_type)
                .then_some(other.pointer_type)
                .unwrap_or_default(),
            save_flag: self.save_flag.diff(&other.save_flag),
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
                .or(self.location_priority),
            message_id: self.message_id.merge(&diff.message_id),
            pointer_type: diff.pointer_type
                .or(self.pointer_type),
            save_flag: self.save_flag.merge(&diff.save_flag),
            show_level: diff.show_level
                .or(self.show_level),
            translate: self.translate.merge(&diff.translate),
        }
    }
}
