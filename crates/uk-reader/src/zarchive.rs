use std::{path::{Path, PathBuf}, sync::Arc};

use serde::Serialize;

use crate::{ROMError, Result};

#[derive(Clone, Debug, Serialize)]
pub(crate) struct ZArchive {
    #[serde(skip_serializing)]
    archive:     Arc<zarchive::reader::ZArchiveReader>,
    content_dir: PathBuf,
    update_dir:  PathBuf,
    aoc_dir:     Option<PathBuf>,
    host_path:   PathBuf,
}

impl ZArchive {
    pub(crate) fn new(path: impl AsRef<Path>) -> Result<Self> {
        log::info!("Opening ZArchive at {}", path.as_ref().display());
        let archive = Arc::new(zarchive::reader::ZArchiveReader::open(path.as_ref())?);
        let mut content_dir: Option<PathBuf> = None;
        let mut update_dir: Option<PathBuf> = None;
        let mut aoc_dir: Option<PathBuf> = None;
        for dir in archive.iter()? {
            if dir.name().starts_with("0005000") && dir.name().ends_with("v0") {
                content_dir = Some(Path::new(dir.name()).join("content"));
                log::debug!("Found content folder in ZArchive at {:?}", &content_dir);
            } else if dir.name().starts_with("0005000") && dir.name().ends_with("v208") {
                update_dir = Some(Path::new(dir.name()).join("content"));
                log::debug!("Found update folder in ZArchive at {:?}", &update_dir);
            } else if dir.name().starts_with("0005000") && dir.name().ends_with("v80") {
                aoc_dir = Some(Path::new(dir.name()).join("content/0010"));
                log::debug!("Found DLC folder in ZArchive at {:?}", &aoc_dir);
            }
        }
        Ok(Self {
            archive,
            content_dir: content_dir.ok_or_else(|| {
                ROMError::MissingDumpDir("base game", path.as_ref().to_path_buf())
            })?,
            update_dir: update_dir
                .ok_or_else(|| ROMError::MissingDumpDir("update", path.as_ref().to_path_buf()))?,
            aoc_dir,
            host_path: path.as_ref().to_path_buf(),
        })
    }
}

#[typetag::serde]
impl super::ResourceLoader for ZArchive {
    fn get_base_file_data(&self, name: &Path) -> Result<Vec<u8>> {
        self.archive.read_file(self.content_dir.join(name)).ok_or_else(|| {
            crate::ROMError::FileNotFound(
                name.to_string_lossy().into(),
                self.host_path.clone(),
            )
        })
    }

    fn get_update_file_data(&self, name: &Path) -> Result<Vec<u8>> {
        self.archive.read_file(self.update_dir.join(name)).ok_or_else(|| {
            crate::ROMError::FileNotFound(
                name.to_string_lossy().into(),
                self.host_path.clone(),
            )
        })
    }

    fn get_aoc_file_data(&self, name: &Path) -> Result<Vec<u8>> {
        self.aoc_dir
            .as_ref()
            .map(|dir| {
                self.archive.read_file(dir.join(name)).ok_or_else(|| {
                    crate::ROMError::FileNotFound(
                        name.to_string_lossy().into(),
                        self.host_path.clone(),
                    )
                })
            })
            .unwrap_or_else(|| {
                Err(crate::ROMError::MissingDumpDir(
                    "DLC",
                    self.host_path.clone(),
                ))
            })
    }

    fn file_exists(&self, name: &Path) -> bool {
        self.archive.file_size(self.update_dir.join(name)).is_some()
            || self
                .archive
                .file_size(self.content_dir.join(name))
                .is_some()
            || self
                .aoc_dir
                .as_ref()
                .map(|aoc| self.archive.file_size(aoc.join(name)).is_some())
                .unwrap_or(false)
    }

    fn host_path(&self) -> &Path {
        &self.host_path
    }
}

mod de {
    use std::{fmt, path::PathBuf};

    use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};

    use super::*;

    impl<'de> Deserialize<'de> for ZArchive {
        fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum Field {
                content_dir,
                update_dir,
                aoc_dir,
                host_path,
            }

            impl<'de> Deserialize<'de> for Field {
                fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
                where
                    D: Deserializer<'de>,
                {
                    struct FieldVisitor;

                    impl<'de> Visitor<'de> for FieldVisitor {
                        type Value = Field;

                        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                            write!(f, "`content_dir`, `update_dir`, `aoc_dir`, or `host_path`")
                        }

                        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            match v {
                                "content_dir" => Ok(Field::content_dir),
                                "update_dir" => Ok(Field::update_dir),
                                "aoc_dir" => Ok(Field::aoc_dir),
                                "host_path" => Ok(Field::host_path),
                                _ => Err(serde::de::Error::custom(format!("unknown field: {}", v))),
                            }
                        }
                    }
                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct ZArchiveVisitor;

            impl<'de> Visitor<'de> for ZArchiveVisitor {
                type Value = ZArchive;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "struct ZArchive")
                }

                fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut content_dir: Option<PathBuf> = None;
                    let mut update_dir: Option<PathBuf> = None;
                    let mut aoc_dir: Option<PathBuf> = None;
                    let mut host_path: Option<PathBuf> = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::content_dir => {
                                content_dir = Some(map.next_value()?);
                            }
                            Field::update_dir => {
                                update_dir = Some(map.next_value()?);
                            }
                            Field::aoc_dir => {
                                aoc_dir = Some(map.next_value()?);
                            }
                            Field::host_path => {
                                host_path = Some(map.next_value()?);
                            }
                        }
                    }
                    let content_dir = content_dir
                        .ok_or_else(|| serde::de::Error::missing_field("content_dir"))?;
                    let update_dir =
                        update_dir.ok_or_else(|| serde::de::Error::missing_field("update_dir"))?;
                    let host_path =
                        host_path.ok_or_else(|| serde::de::Error::missing_field("host_path"))?;
                    Ok(ZArchive {
                        archive: Arc::new(::zarchive::reader::ZArchiveReader::open(&host_path)
                            .map_err(serde::de::Error::custom)?),
                        content_dir,
                        update_dir,
                        aoc_dir,
                        host_path,
                    })
                }
            }

            const FIELDS: &[&str] = &["content_dir", "update_dir", "aoc_dir", "host_path"];
            deserializer.deserialize_struct("ZArchive", FIELDS, ZArchiveVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ResourceLoader;

    #[test]
    fn test_wua() {
        use super::*;
        let arch = ZArchive::new("test/test.wua").unwrap();
        for dir in arch.archive.iter().unwrap() {
            println!("{}", dir.name());
        }
        assert_eq!(
            "0.9.0".to_string(),
            String::from_utf8(arch.get_base_file_data("System/Version.txt".as_ref()).unwrap()).unwrap()
        );
    }
}
