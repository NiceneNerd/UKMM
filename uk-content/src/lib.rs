#![feature(let_chains, type_alias_impl_trait, drain_filter, arbitrary_self_types)]
#![allow(clippy::derive_partial_eq_without_eq)]
use std::path::Path;
use thiserror::Error;

pub mod actor;
pub mod chemical;
pub mod constants;
pub mod cooking;
pub mod data;
pub mod demo;
pub mod eco;
pub mod event;
pub mod hashes;
pub mod map;
pub mod message;
pub mod quest;
pub mod resource;
pub mod sound;
pub mod tips;
pub mod util;
pub mod worldmgr;

#[derive(Debug, Error)]
pub enum UKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(&'static str),
    #[error("Parameter file missing key: {0}")]
    MissingAampKeyD(String),
    #[error("BYML file missing key: {0}")]
    MissingBymlKey(&'static str),
    #[error("BYML file missing key: {0}")]
    MissingBymlKeyD(String),
    #[error("Wrong type for parameter value")]
    WrongAampType(#[from] roead::aamp::AampError),
    #[error("Wrong type for BYML value")]
    WrongBymlType(#[from] roead::byml::BymlError),
    #[error("{0} missing from SARC")]
    MissingSarcFile(&'static str),
    #[error("{0} missing from SARC")]
    MissingSarcFileD(String),
    #[error("Invalid SARC file: {0}")]
    InvalidSarc(#[from] roead::sarc::SarcError),
    #[error("Invalid weather value: {0}")]
    InvalidWeatherOrTime(String),
    #[error("Missing resource at {0}")]
    MissingResource(String),
    #[error("{0}")]
    Other(&'static str),
    #[error("{0}")]
    OtherD(String),
    #[error(transparent)]
    _Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, UKError>;
pub type Assets = util::DeleteMap<String, Vec<u8>>;

pub const fn platform_content(endian: prelude::Endian) -> &'static str {
    match endian {
        prelude::Endian::Little => "01007EF00011E000/romfs",
        prelude::Endian::Big => "content",
    }
}

pub const fn platform_aoc(endian: prelude::Endian) -> &'static str {
    match endian {
        prelude::Endian::Little => "01007EF00011F001/romfs",
        prelude::Endian::Big => "aoc/0010",
    }
}

pub const fn platform_prefixes(endian: prelude::Endian) -> (&'static str, &'static str) {
    match endian {
        prelude::Endian::Little => ("01007EF00011E000/romfs", "01007EF00011F001/romfs"),
        prelude::Endian::Big => ("content", "aoc/0010"),
    }
}

pub fn canonicalize(path: impl AsRef<Path>) -> String {
    let path = path.as_ref().to_str().unwrap();
    let mut canon = path.replace('\\', "/");
    for (k, v) in [
        ("Content/", ""),
        ("content/", ""),
        ("atmosphere/titles/", ""),
        ("atmosphere/contents/", ""),
        ("01007EF00011E000/romfs/", ""),
        ("01007ef00011e000/romfs/", ""),
        ("01007EF00011E001/romfs", "Aoc/0010"),
        ("01007EF00011E002/romfs", "Aoc/0010"),
        ("01007EF00011F001/romfs", "Aoc/0010"),
        ("01007EF00011F002/romfs", "Aoc/0010"),
        ("01007ef00011e001/romfs", "Aoc/0010"),
        ("01007ef00011e002/romfs", "Aoc/0010"),
        ("01007ef00011f001/romfs", "Aoc/0010"),
        ("01007ef00011f002/romfs", "Aoc/0010"),
        ("romfs/", ""),
        ("aoc/content", "Aoc"),
        ("aoc", "Aoc"),
    ]
    .into_iter()
    {
        if canon.starts_with(k) {
            canon = [v, canon.trim_start_matches(k)].concat();
        }
    }
    canon.replace(".s", ".")
}

pub mod prelude {
    use cow_utils::CowUtils;
    use std::borrow::Cow;

    pub trait Mergeable {
        #[must_use]
        fn diff(&self, other: &Self) -> Self;
        #[must_use]
        fn merge(&self, diff: &Self) -> Self;
    }

    macro_rules! impl_simple_aamp {
        ($type:ty, $field:tt) => {
            impl Mergeable for $type {
                fn diff(&self, other: &Self) -> Self {
                    crate::util::diff_plist(&self.$field, &other.$field).into()
                }

                fn merge(&self, diff: &Self) -> Self {
                    let mut pio = crate::util::merge_plist(&self.$field, &diff.$field);
                    pio.doc_type = self.$field.doc_type.clone();
                    pio.version = self.$field.version;
                    pio.into()
                }
            }
        };
    }

    impl Mergeable for roead::aamp::ParameterIO {
        fn diff(&self, other: &Self) -> Self {
            crate::util::diff_plist(self, other)
        }

        fn merge(&self, diff: &Self) -> Self {
            let mut pio = crate::util::merge_plist(self, diff);
            pio.doc_type = self.doc_type.clone();
            pio.version = self.version;
            pio
        }
    }

    pub(crate) use impl_simple_aamp;

    macro_rules! impl_simple_byml {
        ($type:ty, $field:tt) => {
            impl Mergeable for $type {
                fn diff(&self, other: &Self) -> Self {
                    crate::util::diff_byml_shallow(&self.$field, &other.$field).into()
                }

                fn merge(&self, diff: &Self) -> Self {
                    crate::util::merge_byml_shallow(&self.$field, &diff.$field).into()
                }
            }
        };
    }

    impl Mergeable for roead::byml::Byml {
        fn diff(&self, other: &Self) -> Self {
            crate::util::diff_byml_shallow(self, other)
        }

        fn merge(&self, diff: &Self) -> Self {
            crate::util::merge_byml_shallow(self, diff)
        }
    }

    pub(crate) use impl_simple_byml;
    use join_str::jstr;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum Endian {
        Little,
        Big,
    }

    impl From<roead::Endian> for Endian {
        fn from(endian: roead::Endian) -> Self {
            match endian {
                roead::Endian::Little => Endian::Little,
                roead::Endian::Big => Endian::Big,
            }
        }
    }

    impl From<Endian> for roead::Endian {
        fn from(endian: Endian) -> Self {
            match endian {
                Endian::Little => roead::Endian::Little,
                Endian::Big => roead::Endian::Big,
            }
        }
    }

    impl From<&roead::Endian> for Endian {
        fn from(endian: &roead::Endian) -> Self {
            match endian {
                roead::Endian::Little => Endian::Little,
                roead::Endian::Big => Endian::Big,
            }
        }
    }

    impl From<&Endian> for roead::Endian {
        fn from(endian: &Endian) -> Self {
            match endian {
                Endian::Little => roead::Endian::Little,
                Endian::Big => roead::Endian::Big,
            }
        }
    }

    #[derive(Debug)]
    pub enum Binary {
        Bytes(roead::Bytes),
        Vec(Vec<u8>),
    }

    impl PartialEq for Binary {
        fn eq(&self, other: &Self) -> bool {
            self.as_ref() == other.as_ref()
        }
    }

    impl Clone for Binary {
        fn clone(&self) -> Self {
            match self {
                Binary::Bytes(b) => Binary::Vec(b.to_vec()),
                Binary::Vec(v) => Binary::Vec(v.clone()),
            }
        }
    }

    impl std::borrow::Borrow<[u8]> for Binary {
        fn borrow(&self) -> &[u8] {
            match self {
                Binary::Bytes(b) => b.as_slice(),
                Binary::Vec(v) => v.as_slice(),
            }
        }
    }

    impl std::ops::Deref for Binary {
        type Target = [u8];

        fn deref(&self) -> &Self::Target {
            match self {
                Binary::Bytes(b) => b.as_slice(),
                Binary::Vec(v) => v.as_slice(),
            }
        }
    }

    impl AsRef<[u8]> for Binary {
        fn as_ref(&self) -> &[u8] {
            match self {
                Binary::Bytes(b) => b.as_ref(),
                Binary::Vec(v) => v.as_slice(),
            }
        }
    }

    impl From<roead::Bytes> for Binary {
        fn from(b: roead::Bytes) -> Self {
            Binary::Bytes(b)
        }
    }

    impl From<Vec<u8>> for Binary {
        fn from(vec: Vec<u8>) -> Self {
            Binary::Vec(vec)
        }
    }

    impl From<Binary> for Vec<u8> {
        fn from(binary: Binary) -> Self {
            match binary {
                Binary::Bytes(b) => b.to_vec(),
                Binary::Vec(v) => v,
            }
        }
    }

    impl From<roead::yaz0::YazData<'_>> for Binary {
        fn from(data: roead::yaz0::YazData) -> Self {
            match data {
                roead::yaz0::YazData::Borrowed(b) => Binary::Vec(b.to_vec()),
                roead::yaz0::YazData::Owned(b) => Binary::Bytes(b),
            }
        }
    }

    impl From<&[u8]> for Binary {
        fn from(data: &[u8]) -> Self {
            Binary::Vec(data.to_vec())
        }
    }

    impl serde::Serialize for Binary {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.as_ref().serialize(serializer)
        }
    }

    impl<'de> serde::Deserialize<'de> for Binary {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            let bytes = <&[u8]>::deserialize(deserializer)?;
            Ok(Binary::Vec(bytes.to_vec()))
        }
    }

    pub trait Resource
    where
        Self: std::marker::Sized,
    {
        fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self>;
        fn into_binary(self, endian: Endian) -> roead::Bytes;
        fn path_matches(path: impl AsRef<std::path::Path>) -> bool;
    }

    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum ResourceLocation {
        #[default]
        Content,
        Aoc,
    }

    #[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub struct ResourcePath {
        pub path: Cow<'static, str>,
        pub location: ResourceLocation,
    }

    impl ResourcePath {
        pub fn canonical(&self) -> Cow<str> {
            let path = if let Some(pos) = self.path.find("//") {
                &self.path[pos + 2..]
            } else {
                self.path.as_ref()
            };
            return match self.location {
                ResourceLocation::Content => {
                    if path.ends_with(".sarc") {
                        path.into()
                    } else {
                        path.cow_replace(".s", ".")
                    }
                }
                ResourceLocation::Aoc => jstr!("Aoc/0010/{path}").into(),
            };
        }
    }

    pub trait SingleResource: Resource {
        fn path() -> &'static ResourcePath;
    }

    macro_rules! single_path {
        ($type:ty, $path:expr) => {
            static PATH: crate::prelude::ResourcePath = crate::prelude::ResourcePath {
                path: std::borrow::Cow::Borrowed($path),
                location: ResourceLocation::Content,
            };

            impl SingleResource for $type {
                fn path() -> &'static crate::prelude::ResourcePath {
                    &PATH
                }
            }
        };

        ($type:ty, $path:expr, aoc) => {
            static PATH: crate::prelude::ResourcePath = crate::prelude::ResourcePath {
                path: std::borrow::Cow::Borrowed($path),
                location: ResourceLocation::Aoc,
            };

            impl SingleResource for $type {
                fn path() -> &'static crate::prelude::ResourcePath {
                    &PATH
                }
            }
        };
    }

    pub(crate) use single_path;
}

#[cfg(test)]
pub(crate) mod tests {
    use join_str::jstr;

    use crate::canonicalize;

    pub fn test_base_actorpack(name: &str) -> roead::sarc::Sarc<'static> {
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(
                std::fs::read(&jstr!("test/Actor/Pack/{name}.sbactorpack")).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    pub fn test_mod_actorpack(name: &str) -> roead::sarc::Sarc<'static> {
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(
                std::fs::read(&jstr!("test/Actor/Pack/{name}_Mod.sbactorpack")).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn canon_names() {
        assert_eq!(
            &canonicalize("content\\Actor\\Pack\\Enemy_Lizal_Senior.sbactorpack"),
            "Actor/Pack/Enemy_Lizal_Senior.bactorpack"
        );
        assert_eq!(
            &canonicalize("aoc/0010/Map/MainField/A-1/A-1_Dynamic.smubin"),
            "Aoc/0010/Map/MainField/A-1/A-1_Dynamic.mubin"
        );
        assert_eq!(
            &canonicalize(
                "atmosphere/contents/01007EF00011E000/romfs/Actor/ActorInfo.product.sbyml"
            ),
            "Actor/ActorInfo.product.byml"
        );
        assert_eq!(
            &canonicalize("atmosphere/contents/01007EF00011F001/romfs/Pack/AocMainField.pack"),
            "Aoc/0010/Pack/AocMainField.pack"
        );
        assert_eq!(
            &canonicalize("Hellow/Sweetie.tardis"),
            "Hellow/Sweetie.tardis"
        );
        assert_eq!(
            &canonicalize("Event/EventInfo.product.sbyml"),
            "Event/EventInfo.product.byml"
        )
    }
}
