use anyhow::Context;
use join_str::jstr;
use roead::{
    aamp::hash_name,
    byml::{map, Byml},
    sarc::{Sarc, SarcWriter},
};
use serde::{Deserialize, Serialize};
use uk_content_derive::BymlData;

use crate::{
    prelude::*,
    util::{HashMap, SortedDeleteSet},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize, BymlData)]
pub struct SaveDataHeader {
    #[name = "IsCommon"]
    pub is_common: bool,
    #[name = "IsCommonAtSameAccount"]
    pub is_common_at_same_account: bool,
    #[name = "IsSaveSecureCode"]
    pub is_save_secure_code: bool,
    pub file_name: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, BymlData)]
pub struct Flag {
    #[name = "DataName"]
    pub name: String,
    #[name = "HashValue"]
    pub hash: i32,
}

impl From<String> for Flag {
    fn from(name: String) -> Self {
        let hash = hash_name(&name) as i32;
        Self { name, hash }
    }
}

impl From<&str> for Flag {
    fn from(name: &str) -> Self {
        let hash = hash_name(name) as i32;
        Self {
            name: name.into(),
            hash,
        }
    }
}

impl PartialEq for Flag {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for Flag {}

impl std::hash::Hash for Flag {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        state.write_i32(self.hash)
    }
}

impl PartialOrd for Flag {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.hash.cmp(&other.hash))
    }
}

impl Ord for Flag {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.hash.cmp(&other.hash)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct SaveData {
    pub header: SaveDataHeader,
    pub flags:  SortedDeleteSet<Flag>,
}

impl TryFrom<&Byml> for SaveData {
    type Error = UKError;

    fn try_from(val: &Byml) -> Result<Self> {
        let array = val
            .as_map()?
            .get("file_list")
            .ok_or(UKError::MissingBymlKey("bgsvdata missing file_list"))?
            .as_array()?;
        Ok(Self {
            header: array
                .first()
                .ok_or(UKError::MissingBymlKey("bgsvdata missing header"))?
                .try_into()?,
            flags:  array
                .get(1)
                .ok_or(UKError::MissingBymlKey("bgsvdata missing flag array"))?
                .as_array()?
                .iter()
                .map(|flag| -> Result<Flag> {
                    Ok(flag
                        .try_into()
                        .with_context(|| format!("Failed to parse flag: {:?}", flag))?)
                })
                .collect::<Result<SortedDeleteSet<_>>>()?,
        })
    }
}

impl From<SaveData> for Byml {
    fn from(val: SaveData) -> Self {
        map!(
            "file_list" => [
                val.header.into(),
                val.flags.into_iter().map(Byml::from).collect::<Byml>(),
            ]
            .into_iter()
            .collect::<Byml>(),
            "save_info" => Byml::Array(vec![
                map!(
                    "directory_num" => Byml::I32(8),
                    "is_build_machine" => Byml::Bool(true),
                    "revision" => Byml::I32(18203),
                )
            ])
        )
    }
}

impl Mergeable for SaveData {
    fn diff(&self, other: &Self) -> Self {
        assert_eq!(
            self.header, other.header,
            "Attempted to diff incompatible savedata files: {:?} and {:?}",
            self.header, other.header
        );
        Self {
            header: self.header.clone(),
            flags:  self.flags.diff(&other.flags),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        assert_eq!(
            self.header, diff.header,
            "Attempted to merge incompatible savedata files: {:?} and {:?}",
            self.header, diff.header
        );
        Self {
            header: self.header.clone(),
            flags:  self.flags.merge(&diff.flags),
        }
    }
}

impl SaveData {
    fn divide(self) -> Vec<Self> {
        let total = (self.flags.len() as f32 / 8192.).ceil() as usize;
        let mut iter = self.flags.into_iter();
        let mut out = Vec::with_capacity(total);
        for _ in 0..total {
            out.push(Self {
                header: self.header.clone(),
                flags:  iter.by_ref().take(8192).collect(),
            });
        }
        out
    }
}

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]

pub struct SaveDataPack(pub HashMap<String, SaveData>);

impl SaveDataPack {
    pub fn from_sarc(sarc: &Sarc<'_>) -> Result<SaveDataPack> {
        sarc.files()
            .filter(|f| f.name().map(|n| n.ends_with(".bgsvdata")).unwrap_or(false))
            .try_fold(Self(HashMap::default()), |mut acc, file| {
                let byml = Byml::from_binary(file.data())?;
                let savedata = SaveData::try_from(&byml)?;
                let save_file = &savedata.header.file_name;
                if let Some(save_file_data) = acc.0.get_mut(save_file) {
                    *save_file_data = save_file_data.merge(&savedata);
                } else {
                    acc.0.insert(save_file.clone(), savedata);
                }
                Ok(acc)
            })
    }

    pub fn from_sarc_writer(sarc: &SarcWriter) -> Result<SaveDataPack> {
        sarc.files
            .iter()
            .filter(|(f, _)| f.ends_with(".bgsvdata"))
            .try_fold(Self(HashMap::default()), |mut acc, (_, data)| {
                let byml = Byml::from_binary(data)?;
                let savedata = SaveData::try_from(&byml)?;
                let save_file = &savedata.header.file_name;
                if let Some(save_file_data) = acc.0.get_mut(save_file) {
                    *save_file_data = save_file_data.merge(&savedata);
                } else {
                    acc.0.insert(save_file.clone(), savedata);
                }
                Ok(acc)
            })
    }

    pub fn into_sarc_writer(mut self, endian: Endian) -> SarcWriter {
        let mut out = SarcWriter::new(endian.into());
        out.set_min_alignment(4);
        if let Some(game) = self.0.remove("game_data.sav") {
            out.add_files(game.divide().into_iter().enumerate().map(|(i, data)| {
                let name = jstr!("/saveformat_{&lexical::to_string(i)}.bgsvdata");
                (name, Byml::from(data).to_binary(endian.into()))
            }));
        }
        if let Some(caption) = self.0.remove("caption.sav") {
            let count = out.files.len();
            out.add_files(caption.divide().into_iter().enumerate().map(|(i, data)| {
                let name = jstr!("/saveformat_{&lexical::to_string(i + count)}.bgsvdata");
                (name, Byml::from(data).to_binary(endian.into()))
            }));
        }
        if let Some(option) = self.0.remove("option.sav") {
            let count = out.files.len();
            out.add_files(option.divide().into_iter().enumerate().map(|(i, data)| {
                let name = jstr!("/saveformat_{&lexical::to_string(i + count)}.bgsvdata");
                (name, Byml::from(data).to_binary(endian.into()))
            }));
        }
        out
    }
}

impl Mergeable for SaveDataPack {
    fn diff(&self, other: &Self) -> Self {
        Self(
            ["game_data.sav", "caption.sav", "option.sav"]
                .into_iter()
                .map(|key| {
                    (
                        key.into(),
                        self.0
                            .get(key)
                            .unwrap_or(&Default::default())
                            .diff(other.0.get(key).unwrap_or(&Default::default())),
                    )
                })
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(
            ["game_data.sav", "caption.sav", "option.sav"]
                .into_iter()
                .map(|key| {
                    (
                        key.into(),
                        self.0
                            .get(key)
                            .unwrap_or(&Default::default())
                            .merge(diff.0.get(key).unwrap_or(&Default::default())),
                    )
                })
                .collect(),
        )
    }
}

impl Resource for SaveDataPack {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        Self::from_sarc(&Sarc::new(data.as_ref())?)
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        self.into_sarc_writer(endian).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("savedataformat")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::{byml::Byml, sarc::Sarc};

    use crate::prelude::*;

    fn load_savedata_sarc() -> Sarc<'static> {
        Sarc::new(std::fs::read("test/GameData/savedataformat.ssarc").unwrap()).unwrap()
    }

    fn load_savedata() -> Byml {
        let sv = load_savedata_sarc();
        Byml::from_binary(sv.get_data("/saveformat_0.bgsvdata").unwrap()).unwrap()
    }

    fn load_mod_savedata() -> Byml {
        let sv = load_savedata_sarc();
        Byml::from_binary(sv.get_data("/saveformat_0.mod.bgsvdata").unwrap()).unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_savedata();
        let savedata = super::SaveData::try_from(&byml).unwrap();
        let data = Byml::from(savedata.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(data).unwrap();
        let savedata2 = super::SaveData::try_from(&byml2).unwrap();
        assert_eq!(savedata, savedata2);
    }

    #[test]
    fn diff() {
        let byml = load_savedata();
        let savedata = super::SaveData::try_from(&byml).unwrap();
        let byml2 = load_mod_savedata();
        let savedata2 = super::SaveData::try_from(&byml2).unwrap();
        let diff = savedata.diff(&savedata2);
        dbg!(diff);
    }

    #[test]
    fn merge() {
        let byml = load_savedata();
        let savedata = super::SaveData::try_from(&byml).unwrap();
        let byml2 = load_mod_savedata();
        let savedata2 = super::SaveData::try_from(&byml2).unwrap();
        let diff = savedata.diff(&savedata2);
        let merged = savedata.merge(&diff);
        assert_eq!(merged, savedata2);
    }

    #[test]
    fn pack() {
        let pack = super::SaveDataPack::from_sarc(&load_savedata_sarc()).unwrap();
        let pack2 =
            super::SaveDataPack::from_sarc_writer(&pack.clone().into_sarc_writer(Endian::Big))
                .unwrap();
        assert_eq!(pack, pack2);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new("content/Pack/Bootup.pack//GameData/savedataformat.ssarc");
        assert!(super::SaveDataPack::path_matches(path));
    }
}
