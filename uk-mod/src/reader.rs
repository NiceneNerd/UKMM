use crate::{Manifest, Meta};
use anyhow::{Context, Result};
use fs_err::File;
use join_str::jstr;
use parking_lot::Mutex;
use serde::Serialize;
use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::Arc,
};
use uk_content::canonicalize;
use uk_reader::ResourceLoader;
use zip::ZipArchive;

type ZipReader = Arc<Mutex<ZipArchive<BufReader<File>>>>;

#[derive(Debug, Serialize)]
pub struct ModReader {
    path: PathBuf,
    meta: Meta,
    manifest: Manifest,
    #[serde(skip_serializing)]
    zip: ZipReader,
}

#[typetag::serde]
impl ResourceLoader for ModReader {
    fn file_exists(&self, name: &Path) -> bool {
        let name = name.to_string_lossy();
        self.manifest.content_files.contains(name.as_ref())
            || self.manifest.aoc_files.contains(name.as_ref())
    }

    fn get_file_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(name);
        Ok(self
            .zip
            .lock()
            .by_name(&canon)
            .map_err(anyhow::Error::from)?
            .bytes()
            .map(|b| b.map_err(anyhow::Error::from))
            .into_iter()
            .collect::<Result<_>>()?)
    }

    fn get_aoc_file_data(&self, name: &Path) -> uk_reader::Result<Vec<u8>> {
        let canon = canonicalize(jstr!("Aoc/0010/{name.to_str().unwrap_or_default()}"));
        Ok(self
            .zip
            .lock()
            .by_name(&canon)
            .map_err(anyhow::Error::from)?
            .bytes()
            .map(|b| b.map_err(anyhow::Error::from))
            .into_iter()
            .collect::<Result<_>>()?)
    }

    fn host_path(&self) -> &Path {
        &self.path
    }
}

impl ModReader {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut zip = ZipArchive::new(BufReader::new(File::open(&path)?))?;
        let manifest: Manifest = yaml_peg::serde::from_str(std::str::from_utf8(
            &zip.by_name("manifest.yml")
                .context("Mod missing manifest")?
                .bytes()
                .map(|b| b.map_err(anyhow::Error::from))
                .collect::<Result<Vec<u8>>>()?,
        )?)?
        .swap_remove(0);
        let meta: Meta = toml::from_slice(
            &zip.by_name("meta.toml")
                .context("Mod missing meta")?
                .bytes()
                .map(|b| b.map_err(anyhow::Error::from))
                .collect::<Result<Vec<u8>>>()?,
        )?;
        Ok(Self {
            path,
            meta,
            manifest,
            zip: Arc::new(Mutex::new(zip)),
        })
    }

    pub fn meta(&self) -> &Meta {
        &self.meta
    }
}
mod de {
    use super::*;
    use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};
    use std::fmt;
    use std::path::PathBuf;

    impl<'de> Deserialize<'de> for ModReader {
        fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            enum Field {
                path,
                meta,
                manifest,
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
                            write!(f, "`path`, `meta`, or `manifest`")
                        }

                        fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
                        where
                            E: serde::de::Error,
                        {
                            match v {
                                "path" => Ok(Field::path),
                                "meta" => Ok(Field::meta),
                                "manifest" => Ok(Field::manifest),
                                _ => Err(serde::de::Error::custom(format!("unknown field: {}", v))),
                            }
                        }
                    }
                    deserializer.deserialize_identifier(FieldVisitor)
                }
            }

            struct ModReaderVisitor;

            impl<'de> Visitor<'de> for ModReaderVisitor {
                type Value = ModReader;

                fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                    write!(f, "struct ModReader")
                }

                fn visit_map<A>(self, mut map: A) -> std::result::Result<Self::Value, A::Error>
                where
                    A: MapAccess<'de>,
                {
                    let mut path: Option<PathBuf> = None;
                    let mut meta: Option<Meta> = None;
                    let mut manifest: Option<Manifest> = None;
                    while let Some(key) = map.next_key()? {
                        match key {
                            Field::path => {
                                path = Some(map.next_value()?);
                            }
                            Field::meta => {
                                meta = Some(map.next_value()?);
                            }
                            Field::manifest => {
                                manifest = Some(map.next_value()?);
                            }
                        }
                    }
                    let path = path.ok_or_else(|| serde::de::Error::missing_field("path"))?;
                    let meta = meta.ok_or_else(|| serde::de::Error::missing_field("meta"))?;
                    let manifest =
                        manifest.ok_or_else(|| serde::de::Error::missing_field("manifest"))?;
                    Ok(ModReader {
                        meta,
                        manifest,
                        zip: Arc::new(Mutex::new(
                            ZipArchive::new(BufReader::new(
                                File::open(&path).map_err(serde::de::Error::custom)?,
                            ))
                            .map_err(serde::de::Error::custom)?,
                        )),
                        path,
                    })
                }
            }

            const FIELDS: &[&str] = &["path", "meta", "manifest"];
            deserializer.deserialize_struct("ModReader", FIELDS, ModReaderVisitor)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn open_mod() {
        let mod_reader = ModReader::open("test/wiiu.zip").unwrap();
        dbg!(&mod_reader.manifest);
        dbg!(mod_reader.meta);
        dbg!(mod_reader.manifest.resources().collect::<Vec<String>>());
    }
}
