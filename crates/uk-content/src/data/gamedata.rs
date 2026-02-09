use std::hint::unreachable_unchecked;

use anyhow::Context;
use join_str::jstr;
use lighter::lighter;
use roead::{
    byml::{map, Byml},
    sarc::{Sarc, SarcWriter},
};
use serde::{Deserialize, Serialize};
use uk_content_derive::BymlData;

use crate::{prelude::*, util::DeleteMap, Result, UKError};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, BymlData)]
pub struct FlagData {
    #[name = "Category"]
    category: Option<i32>,
    #[name = "DataName"]
    pub data_name: String,
    #[name = "DeleteRev"]
    delete_rev: i32,
    #[name = "HashValue"]
    hash_value: i32,
    #[name = "InitValue"]
    init_value: Byml,
    #[name = "IsEventAssociated"]
    is_event_associated: bool,
    #[name = "IsOneTrigger"]
    is_one_trigger: bool,
    #[name = "IsProgramReadable"]
    is_program_readable: bool,
    #[name = "IsProgramWritable"]
    is_program_writable: bool,
    #[name = "IsSave"]
    is_save: bool,
    #[name = "MaxValue"]
    max_value: Byml,
    #[name = "MinValue"]
    min_value: Byml,
    #[name = "ResetType"]
    reset_type: i32,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct GameData {
    pub data_type: String,
    pub flags:     DeleteMap<String, FlagData>,
}

impl TryFrom<&Byml> for GameData {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_map()?;
        Ok(Self {
            data_type: hash
                .keys()
                .next()
                .ok_or(UKError::MissingBymlKey("bgdata file missing data type key"))?
                .clone(),
            flags:     hash
                .values()
                .next()
                .ok_or(UKError::MissingBymlKey("bgdata file missing data"))?
                .as_array()?
                .iter()
                .map(|item| -> Result<(String, FlagData)> {
                    let name = item
                        .as_map()?
                        .get("DataName")
                        .ok_or(UKError::MissingBymlKey("Gamedata flag missing DataName"))?
                        .as_string()?;
                    Ok((
                        name.clone(),
                        item.try_into()
                            .with_context(|| jstr!("Failed to parse flag {&name}"))?,
                    ))
                })
                .collect::<Result<_>>()?,
        })
    }
}

impl From<GameData> for Byml {
    fn from(val: GameData) -> Self {
        map!(
            val.data_type => val.flags
            .into_iter()
            .map(|(_, f)| -> Byml { f.into() })
            .collect()
        )
    }
}

impl GameData {
    fn divide(self) -> Vec<GameData> {
        let total = (self.flags.len() as f32 / 4096.).ceil() as usize;
        let mut iter = self.flags.into_iter();
        let mut out = Vec::with_capacity(total);
        for _ in 0..total {
            out.push(GameData {
                data_type: self.data_type.clone(),
                flags:     iter.by_ref().take(4096).collect(),
            });
        }
        out
    }
}

impl Mergeable for GameData {
    fn diff(&self, other: &Self) -> Self {
        assert_eq!(
            self.data_type, other.data_type,
            "Attempted to diff different gamedata types: {} and {}",
            self.data_type, other.data_type
        );
        Self {
            data_type: self.data_type.clone(),
            flags:     self.flags.diff(&other.flags),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        assert_eq!(
            self.data_type, diff.data_type,
            "Attempted to merge different gamedata types: {} and {}",
            self.data_type, diff.data_type
        );
        Self {
            data_type: self.data_type.clone(),
            flags:     self.flags.merge(&diff.flags),
        }
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]
pub struct GameDataPack {
    pub bool_array_data: GameData,
    pub bool_data: GameData,
    pub f32_array_data: GameData,
    pub f32_data: GameData,
    pub revival_bool_data: GameData,
    pub revival_s32_data: GameData,
    pub s32_array_data: GameData,
    pub s32_data: GameData,
    pub string32_data: GameData,
    pub string64_array_data: GameData,
    pub string64_data: GameData,
    pub string256_array_data: GameData,
    pub string256_data: GameData,
    pub vector2f_array_data: GameData,
    pub vector2f_data: GameData,
    pub vector3f_array_data: GameData,
    pub vector3f_data: GameData,
    pub vector4f_data: GameData,
}

#[inline(always)]
#[allow(clippy::needless_borrow)]
fn flag_alloc_count(data_type: &str) -> usize {
    lighter! {
        match data_type {
            "bool_data" => 9564,
            "f32_data" => 46,
            "s32_data" => 2709,
            "string32_data" => 6,
            "string64_data" => 16,
            "string256_data" => 3,
            "vector2f_data" => 1,
            "vector3f_data" => 44,
            "vector4f_data" => 1,
            "bool_array_data" => 5,
            "f32_array_data" => 3,
            "s32_array_data" => 35,
            "string64_array_data" => 28,
            "string256_array_data" => 4,
            "vector2f_array_data" => 3,
            "vector3f_array_data" => 3,
            "revival_bool_data" => 32461,
            "revival_s32_data" => 83,
            _ => unsafe { unreachable_unchecked() },
        }
    }
}

enum SarcSource<'a> {
    Reader(&'a Sarc<'a>),
    Writer(&'a SarcWriter),
}

impl SarcSource<'_> {
    fn iter(&self) -> Box<dyn Iterator<Item = (&str, &[u8])> + '_> {
        match self {
            Self::Reader(sarc) => {
                Box::new(sarc.files().filter_map(|f| f.name.map(|n| (n, f.data))))
            }
            Self::Writer(sarcwriter) => {
                Box::new(
                    sarcwriter
                        .files
                        .iter()
                        .map(|(f, d)| (f.as_ref(), d.as_ref())),
                )
            }
        }
    }

    fn get(&self, file: &str) -> Option<&[u8]> {
        match self {
            Self::Reader(sarc) => sarc.get_data(file),
            Self::Writer(sarcwriter) => sarcwriter.files.get(file).map(|d| d.as_ref()),
        }
    }
}

#[inline]
fn extract_gamedata_by_type(sarc: &SarcSource, key: &str) -> Result<GameData> {
    let data_type = if key == "string32_data" {
        "string_data"
    } else {
        key.trim_start_matches("revival_")
    };
    let mut flags = DeleteMap::with_capacity(flag_alloc_count(key));
    let mut i = 0;
    while let Some(data) = sarc
        .get(&format!("/{key}_{i}.bgdata"))
        .or_else(|| sarc.get(&format!("{key}_{i}.bgdata")))
    {
        let mut hash = Byml::from_binary(data)?.into_map()?;
        if let Some(Byml::Array(arr)) = hash.remove(data_type) {
            for item in arr {
                flags.insert(
                    item.as_map()?
                        .get("DataName")
                        .ok_or(UKError::MissingBymlKey(
                            "bgdata file entry missing DataName",
                        ))?
                        .as_string()?
                        .clone(),
                    (&item).try_into()?,
                );
            }
        }
        i += 1;
    }
    Ok(GameData {
        data_type: data_type.into(),
        flags,
    })
}

macro_rules! build_gamedata_pack {
    ($data:expr, $sarc:expr, $($type:ident),*) => {
        let _endian = $sarc.endian;
        $(
            let _type = $data.$type.data_type.clone();
            let _name_type = stringify!($type);
            $sarc.add_files($data.$type.divide().into_iter().enumerate().map(|(i, data)| {
                (
                    join_str::jstr!("/{_name_type}_{&lexical::to_string(i)}.bgdata"),
                    Byml::from(data).to_binary(_endian),
                )
            }));
        )*
    };
}

impl GameDataPack {
    pub const STAGES: &'static [&'static str] =
        &["MainField", "AocField", "CDungeon", "MainFieldDungeon"];

    pub fn from_sarc_writer(sarc: &SarcWriter) -> Result<Self> {
        let source = SarcSource::Writer(sarc);
        if sarc
            .files
            .keys()
            .any(|f| f.trim_start_matches('/').starts_with("revival"))
        {
            Ok(GameDataPack {
                bool_array_data: extract_gamedata_by_type(&source, "bool_array_data")?,
                bool_data: extract_gamedata_by_type(&source, "bool_data")?,
                f32_array_data: extract_gamedata_by_type(&source, "f32_array_data")?,
                f32_data: extract_gamedata_by_type(&source, "f32_data")?,
                revival_bool_data: extract_gamedata_by_type(&source, "revival_bool_data")?,
                revival_s32_data: extract_gamedata_by_type(&source, "revival_s32_data")?,
                s32_array_data: extract_gamedata_by_type(&source, "s32_array_data")?,
                s32_data: extract_gamedata_by_type(&source, "s32_data")?,
                string32_data: extract_gamedata_by_type(&source, "string32_data")?,
                string64_array_data: extract_gamedata_by_type(&source, "string64_array_data")?,
                string64_data: extract_gamedata_by_type(&source, "string64_data")?,
                string256_array_data: extract_gamedata_by_type(&source, "string256_array_data")?,
                string256_data: extract_gamedata_by_type(&source, "string256_data")?,
                vector2f_array_data: extract_gamedata_by_type(&source, "vector2f_array_data")?,
                vector2f_data: extract_gamedata_by_type(&source, "vector2f_data")?,
                vector3f_array_data: extract_gamedata_by_type(&source, "vector3f_array_data")?,
                vector3f_data: extract_gamedata_by_type(&source, "vector3f_data")?,
                vector4f_data: extract_gamedata_by_type(&source, "vector4f_data")?,
            })
        } else {
            Self::from_bcml_sarc(&source)
        }
    }

    fn from_bcml_sarc(sarc: &SarcSource) -> Result<Self> {
        let mut revival_bool_data = DeleteMap::with_capacity(32461);
        let mut bool_data = DeleteMap::with_capacity(9564);
        let mut revival_s32_data = DeleteMap::with_capacity(83);
        let mut s32_data = DeleteMap::with_capacity(2709);
        let mut string32_data = DeleteMap::with_capacity(6);
        for (file, data) in sarc.iter() {
            let file_name = file.trim_start_matches('/');
            let mut byml = Byml::from_binary(data)?;
            let hash = byml.as_mut_map()?;
            if file_name.starts_with("bool_data") {
                if let Some(Byml::Array(arr)) = hash.remove("bool_data") {
                    for item in arr {
                        let name = item
                            .as_map()?
                            .get("DataName")
                            .ok_or(UKError::MissingBymlKey("Game data entry missing DataName"))?
                            .as_string()?
                            .clone();
                        let mut parts = name.split('_');
                        if Self::STAGES.contains(&parts.next().unwrap_or(""))
                            && !name.contains("HiddenKorok")
                        {
                            revival_bool_data.insert(name, (&item).try_into()?);
                        } else {
                            bool_data.insert(name, (&item).try_into()?);
                        }
                    }
                }
            } else if file_name.starts_with("s32_data") {
                if let Some(Byml::Array(arr)) = hash.remove("s32_data") {
                    for item in arr {
                        let name = item
                            .as_map()?
                            .get("DataName")
                            .ok_or(UKError::MissingBymlKey("Game data entry missing DataName"))?
                            .as_string()?
                            .clone();
                        let mut parts = name.split('_');
                        if Self::STAGES.contains(&parts.next().unwrap_or("")) {
                            revival_s32_data.insert(name, (&item).try_into()?);
                        } else {
                            s32_data.insert(name, (&item).try_into()?);
                        }
                    }
                }
            } else if file_name.starts_with("string_data") {
                if let Some(Byml::Array(arr)) = hash.remove("string_data") {
                    for item in arr {
                        let hash_value = item
                            .as_map()?
                            .get("DataName")
                            .ok_or(UKError::MissingBymlKey(
                                "bgdata file entry missing DataName",
                            ))?
                            .as_string()?
                            .clone();
                        string32_data.insert(hash_value, (&item).try_into()?);
                    }
                }
            }
        }
        Ok(GameDataPack {
            bool_data: GameData {
                data_type: "bool_data".into(),
                flags:     bool_data,
            },
            revival_bool_data: GameData {
                data_type: "bool_data".into(),
                flags:     revival_bool_data,
            },
            s32_data: GameData {
                data_type: "s32_data".into(),
                flags:     s32_data,
            },
            revival_s32_data: GameData {
                data_type: "s32_data".into(),
                flags:     revival_s32_data,
            },
            string32_data: GameData {
                data_type: "string_data".into(),
                flags:     string32_data,
            },
            bool_array_data: extract_gamedata_by_type(sarc, "bool_array_data")?,
            s32_array_data: extract_gamedata_by_type(sarc, "s32_array_data")?,
            f32_array_data: extract_gamedata_by_type(sarc, "f32_array_data")?,
            f32_data: extract_gamedata_by_type(sarc, "f32_data")?,
            string64_data: extract_gamedata_by_type(sarc, "string64_data")?,
            string64_array_data: extract_gamedata_by_type(sarc, "string64_array_data")?,
            string256_data: extract_gamedata_by_type(sarc, "string256_data")?,
            string256_array_data: extract_gamedata_by_type(sarc, "string256_array_data")?,
            vector2f_array_data: extract_gamedata_by_type(sarc, "vector2f_array_data")?,
            vector2f_data: extract_gamedata_by_type(sarc, "vector2f_data")?,
            vector3f_array_data: extract_gamedata_by_type(sarc, "vector3f_array_data")?,
            vector3f_data: extract_gamedata_by_type(sarc, "vector3f_data")?,
            vector4f_data: extract_gamedata_by_type(sarc, "vector4f_data")?,
        })
    }

    pub fn from_sarc(sarc: &Sarc<'_>) -> Result<Self> {
        let source = SarcSource::Reader(sarc);
        if sarc.files().any(|f| {
            f.name()
                .unwrap_or("")
                .trim_start_matches('/')
                .starts_with("revival")
        }) {
            Ok(GameDataPack {
                bool_array_data: extract_gamedata_by_type(&source, "bool_array_data")?,
                bool_data: extract_gamedata_by_type(&source, "bool_data")?,
                f32_array_data: extract_gamedata_by_type(&source, "f32_array_data")?,
                f32_data: extract_gamedata_by_type(&source, "f32_data")?,
                revival_bool_data: extract_gamedata_by_type(&source, "revival_bool_data")?,
                revival_s32_data: extract_gamedata_by_type(&source, "revival_s32_data")?,
                s32_array_data: extract_gamedata_by_type(&source, "s32_array_data")?,
                s32_data: extract_gamedata_by_type(&source, "s32_data")?,
                string32_data: extract_gamedata_by_type(&source, "string32_data")?,
                string64_array_data: extract_gamedata_by_type(&source, "string64_array_data")?,
                string64_data: extract_gamedata_by_type(&source, "string64_data")?,
                string256_array_data: extract_gamedata_by_type(&source, "string256_array_data")?,
                string256_data: extract_gamedata_by_type(&source, "string256_data")?,
                vector2f_array_data: extract_gamedata_by_type(&source, "vector2f_array_data")?,
                vector2f_data: extract_gamedata_by_type(&source, "vector2f_data")?,
                vector3f_array_data: extract_gamedata_by_type(&source, "vector3f_array_data")?,
                vector3f_data: extract_gamedata_by_type(&source, "vector3f_data")?,
                vector4f_data: extract_gamedata_by_type(&source, "vector4f_data")?,
            })
        } else {
            Self::from_bcml_sarc(&source)
        }
    }

    pub fn into_sarc_writer(self, endian: Endian) -> SarcWriter {
        let mut sarc = SarcWriter::new(endian.into());
        sarc.set_legacy_mode(true);
        sarc.set_min_alignment(4);
        build_gamedata_pack!(
            self,
            sarc,
            bool_array_data,
            bool_data,
            f32_array_data,
            f32_data,
            revival_bool_data,
            revival_s32_data,
            s32_array_data,
            s32_data,
            string32_data,
            string64_array_data,
            string64_data,
            string256_array_data,
            string256_data,
            vector2f_array_data,
            vector2f_data,
            vector3f_array_data,
            vector3f_data,
            vector4f_data
        );
        sarc
    }
}

impl Mergeable for GameDataPack {
    fn diff(&self, other: &Self) -> Self {
        Self {
            bool_array_data: self.bool_array_data.diff(&other.bool_array_data),
            bool_data: self.bool_data.diff(&other.bool_data),
            f32_array_data: self.f32_array_data.diff(&other.f32_array_data),
            f32_data: self.f32_data.diff(&other.f32_data),
            revival_bool_data: self.revival_bool_data.diff(&other.revival_bool_data),
            revival_s32_data: self.revival_s32_data.diff(&other.revival_s32_data),
            s32_array_data: self.s32_array_data.diff(&other.s32_array_data),
            s32_data: self.s32_data.diff(&other.s32_data),
            string32_data: self.string32_data.diff(&other.string32_data),
            string64_array_data: self.string64_array_data.diff(&other.string64_array_data),
            string64_data: self.string64_data.diff(&other.string64_data),
            string256_array_data: self.string256_array_data.diff(&other.string256_array_data),
            string256_data: self.string256_data.diff(&other.string256_data),
            vector2f_array_data: self.vector2f_array_data.diff(&other.vector2f_array_data),
            vector2f_data: self.vector2f_data.diff(&other.vector2f_data),
            vector3f_array_data: self.vector3f_array_data.diff(&other.vector3f_array_data),
            vector3f_data: self.vector3f_data.diff(&other.vector3f_data),
            vector4f_data: self.vector4f_data.diff(&other.vector4f_data),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            bool_array_data: self.bool_array_data.merge(&diff.bool_array_data),
            bool_data: self.bool_data.merge(&diff.bool_data),
            f32_array_data: self.f32_array_data.merge(&diff.f32_array_data),
            f32_data: self.f32_data.merge(&diff.f32_data),
            revival_bool_data: self.revival_bool_data.merge(&diff.revival_bool_data),
            revival_s32_data: self.revival_s32_data.merge(&diff.revival_s32_data),
            s32_array_data: self.s32_array_data.merge(&diff.s32_array_data),
            s32_data: self.s32_data.merge(&diff.s32_data),
            string32_data: self.string32_data.merge(&diff.string32_data),
            string64_array_data: self.string64_array_data.merge(&diff.string64_array_data),
            string64_data: self.string64_data.merge(&diff.string64_data),
            string256_array_data: self.string256_array_data.merge(&diff.string256_array_data),
            string256_data: self.string256_data.merge(&diff.string256_data),
            vector2f_array_data: self.vector2f_array_data.merge(&diff.vector2f_array_data),
            vector2f_data: self.vector2f_data.merge(&diff.vector2f_data),
            vector3f_array_data: self.vector3f_array_data.merge(&diff.vector3f_array_data),
            vector3f_data: self.vector3f_data.merge(&diff.vector3f_data),
            vector4f_data: self.vector4f_data.merge(&diff.vector4f_data),
        }
    }
}

impl Resource for GameDataPack {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        Self::from_sarc(&Sarc::new(data.as_ref())?)
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        self.into_sarc_writer(endian).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("gamedata")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::{byml::Byml, sarc::Sarc};

    use crate::prelude::*;

    fn load_gamedata_sarc() -> Sarc<'static> {
        Sarc::new(std::fs::read("test/GameData/gamedata.ssarc").unwrap()).unwrap()
    }

    fn load_gamedata() -> Byml {
        let gs = load_gamedata_sarc();
        Byml::from_binary(gs.get_data("/revival_s32_data_0.bgdata").unwrap()).unwrap()
    }

    fn load_mod_gamedata() -> Byml {
        let gs = load_gamedata_sarc();
        Byml::from_binary(gs.get_data("/revival_s32_data_0.mod.bgdata").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let data = Byml::from(gamedata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        assert_eq!(gamedata, gamedata2);
    }

    #[test]
    fn diff() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let byml2 = load_mod_gamedata();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        let diff = gamedata.diff(&gamedata2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_gamedata();
        let gamedata = super::GameData::try_from(&byml).unwrap();
        let byml2 = load_mod_gamedata();
        let gamedata2 = super::GameData::try_from(&byml2).unwrap();
        let diff = gamedata.diff(&gamedata2);
        let merged = gamedata.merge(&diff);
        assert_eq!(merged, gamedata2);
    }

    #[test]
    fn pack() {
        let gs = load_gamedata_sarc();
        let gamedata = super::GameDataPack::from_sarc(&gs).unwrap();
        let gs2 = gamedata
            .clone()
            .into_sarc_writer(Endian::Big);
        let gamedata2 = super::GameDataPack::from_sarc_writer(&gs2).unwrap();
        assert_eq!(gamedata, gamedata2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//GameData/gamedata.ssarc");
        assert!(super::GameDataPack::path_matches(path));
    }
}
