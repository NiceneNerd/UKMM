use roead::byml::Byml;
use smartstring::alias::String;

use crate::{prelude::Mergeable, util::{DeleteVec, HashMap}};

#[derive(Debug, Clone, Default, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct LocationPointer {
    pub location_priority:  Option<i32>,
    pub message_id:         Option<String>,
    pub pointer_type:       Option<i32>,
    pub save_flag:          Option<String>,
    pub show_level:         Option<i32>,
    pub translate:          DeleteVec<(char, f32)>,
}

impl From<&Byml> for LocationPointer {
    fn from(value: &Byml) -> Self {
        let map = value.as_map()
            .expect("TargetPosMarker node must be HashMap");
        Self {
            location_priority: Some(map.get("LocationPriority")
                .expect("LocationPointer must have LocationPriority")
                .as_i32()
                .expect("LocationPointer LocationPriority must be Int")),
            message_id: map.get("MessageID")
                .map(|b| b.as_string()
                    .expect("LocationPointer MessageID must be String")
                    .clone()
                ),
            pointer_type: Some(map.get("PointerType")
                .expect("LocationPointer must have PointerType")
                .as_i32()
                .expect("LocationPointer PointerType must be Int")),
            save_flag: map.get("SaveFlag")
                .map(|b| b.as_string()
                    .expect("LocationPointer SaveFlag must be String")
                    .clone()
                ),
            show_level: Some(map.get("ShowLevel")
                .expect("LocationPointer must have ShowLevel")
                .as_i32()
                .expect("LocationPointer ShowLevel must be Int")),
            translate: map.get("Translate")
                .expect("LocationPointer must have Translate")
                .as_map()
                .expect("Invalid LocationPointer Translate")
                .iter()
                .map(|(k, v)| (
                    k.chars().next().unwrap(),
                    v.as_float().expect("Invalid Float"))
                )
                .collect::<DeleteVec<_>>(),
        }
    }
}

impl From<LocationPointer> for Byml {
    fn from(val: LocationPointer) -> Self {
        let mut map: HashMap<String, Byml> = Default::default();
        map.insert("LocationPriority".into(), val.location_priority.unwrap().into());
        match &val.message_id {
            Some(i) => map.insert("MessageID".into(), i.into()),
            None => None,
        };
        map.insert("PointerType".into(), val.pointer_type.unwrap().into());
        match &val.save_flag {
            Some(i) => map.insert("SaveFlag".into(), i.into()),
            None => None,
        };
        map.insert("ShowLevel".into(), val.show_level.unwrap().into());
        map.insert("Translate".into(), Byml::Map(val.translate
            .iter()
            .map(|(k, v)| (k.to_string().into(), Byml::Float(*v)))
            .collect::<crate::util::HashMap<String, Byml>>()));
        Byml::Map(map)
    }
}

impl Mergeable for LocationPointer {
    fn diff(&self, other: &Self) -> Self {
        Self {
            location_priority: other.location_priority
                .ne(&self.location_priority)
                .then(|| other.location_priority)
                .unwrap(),
            message_id: other.message_id
                .ne(&self.message_id)
                .then(|| other.message_id.clone())
                .unwrap(),
            pointer_type: other.pointer_type
                .ne(&self.pointer_type)
                .then(|| other.pointer_type)
                .unwrap(),
            save_flag: other.save_flag
                .ne(&self.save_flag)
                .then(|| other.save_flag.clone())
                .unwrap(),
            show_level: other.show_level
                .ne(&self.show_level)
                .then(|| other.show_level)
                .unwrap(),
            translate: self.translate.diff(&other.translate),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            location_priority: diff.location_priority
                .or(self.location_priority),
            message_id: diff.message_id.clone()
                .or(self.message_id.clone()),
            pointer_type: diff.pointer_type
                .or(self.pointer_type),
            save_flag: diff.save_flag.clone()
                .or(self.save_flag.clone()),
            show_level: diff.show_level
                .or(self.show_level),
            translate: self.translate.diff(&diff.translate),
        }
    }
}
