#![doc(hidden)]
use std::{fmt, path::PathBuf};

use serde::de::{Deserialize, Deserializer, MapAccess, Visitor};

use super::*;

impl<'de> Deserialize<'de> for ModReader {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[allow(non_camel_case_types)]
        enum Field {
            path,
            options,
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
                        write!(f, "`path`, `options`, `meta`, or `manifest`")
                    }

                    fn visit_str<E>(self, v: &str) -> std::result::Result<Self::Value, E>
                    where
                        E: serde::de::Error,
                    {
                        match v {
                            "path" => Ok(Field::path),
                            "options" => Ok(Field::options),
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
                let mut options: Option<Vec<ModOption>> = None;
                let mut meta: Option<Meta> = None;
                let mut manifest: Option<Manifest> = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::path => {
                            path = Some(map.next_value()?);
                        }
                        Field::options => {
                            options = Some(map.next_value()?);
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
                let options = options.ok_or_else(|| serde::de::Error::missing_field("options"))?;
                let meta = meta.ok_or_else(|| serde::de::Error::missing_field("meta"))?;
                let manifest =
                    manifest.ok_or_else(|| serde::de::Error::missing_field("manifest"))?;
                Ok(ModReader {
                    meta,
                    decompressor: init_decompressor(),
                    manifest,
                    options,
                    zip: Arc::new(Some(
                        ParallelZipReader::open(&path, false)
                            .map_err(serde::de::Error::custom)?,
                    )),
                    path,
                })
            }
        }

        const FIELDS: &[&str] = &["path", "options", "manifest", "meta"];
        deserializer.deserialize_struct("ModReader", FIELDS, ModReaderVisitor)
    }
}
