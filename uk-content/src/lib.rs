#![feature(
    type_alias_impl_trait,
    drain_filter,
    arbitrary_self_types,
    let_chains,
    negative_impls,
    min_specialization
)]
#![allow(clippy::derive_partial_eq_without_eq)]
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
use std::path::Path;

use smartstring::alias::String;
use thiserror::Error;

pub mod actor;
pub mod chemical;
pub mod constants;
pub mod cooking;
pub mod data;
pub mod demo;
pub mod eco;
pub mod event;
pub mod map;
pub mod message;
pub mod quest;
pub mod resource;
pub mod sound;
pub mod tips;
pub mod util;
pub mod worldmgr;

#[derive(Debug, Clone)]
pub enum ContextData {
    Parameter(roead::aamp::Parameter),
    List(roead::aamp::ParameterList),
    Object(roead::aamp::ParameterObject),
    Byml(roead::byml::Byml),
}

impl From<roead::aamp::Parameter> for ContextData {
    fn from(param: roead::aamp::Parameter) -> Self {
        ContextData::Parameter(param)
    }
}

impl From<&roead::aamp::Parameter> for ContextData {
    fn from(param: &roead::aamp::Parameter) -> Self {
        ContextData::Parameter(param.clone())
    }
}

impl From<roead::aamp::ParameterList> for ContextData {
    fn from(list: roead::aamp::ParameterList) -> Self {
        ContextData::List(list)
    }
}

impl From<roead::aamp::ParameterObject> for ContextData {
    fn from(obj: roead::aamp::ParameterObject) -> Self {
        ContextData::Object(obj)
    }
}

impl From<&roead::aamp::ParameterList> for ContextData {
    fn from(list: &roead::aamp::ParameterList) -> Self {
        ContextData::List(list.clone())
    }
}

impl From<&roead::aamp::ParameterObject> for ContextData {
    fn from(obj: &roead::aamp::ParameterObject) -> Self {
        ContextData::Object(obj.clone())
    }
}

impl From<roead::byml::Byml> for ContextData {
    fn from(by: roead::byml::Byml) -> Self {
        ContextData::Byml(by)
    }
}

impl From<&roead::byml::Byml> for ContextData {
    fn from(by: &roead::byml::Byml) -> Self {
        ContextData::Byml(by.clone())
    }
}

#[derive(Debug, Error)]
pub enum UKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(&'static str, Option<ContextData>),
    #[error("Parameter file missing key: {0}")]
    MissingAampKeyD(std::string::String),
    #[error("BYML file missing key: {0}")]
    MissingBymlKey(&'static str),
    #[error("BYML file missing key: {0}")]
    MissingBymlKeyD(std::string::String),
    #[error("Wrong type for BYML value: found {0}, expected {1}")]
    WrongBymlType(std::string::String, &'static str),
    #[error("{0} missing from SARC")]
    MissingSarcFile(&'static str),
    #[error("{0} missing from SARC")]
    MissingSarcFileD(std::string::String),
    #[error("Invalid weather value: {0}")]
    InvalidWeatherOrTime(std::string::String),
    #[error("Missing resource at {0}")]
    MissingResource(std::string::String),
    #[error("{0}")]
    Other(&'static str),
    #[error("{0}")]
    OtherD(std::string::String),
    #[error(transparent)]
    _Infallible(#[from] std::convert::Infallible),
    #[error(transparent)]
    RoeadError(#[from] roead::Error),
    #[error(transparent)]
    Any(#[from] anyhow::Error),
    #[error("Invalid BYML data for field {0}: {1:#?}")]
    InvalidByml(String, roead::byml::Byml),
    #[error("Invalid parameter data for field {0}: {1:#?}")]
    InvalidParameter(String, roead::aamp::Parameter),
}

impl UKError {
    pub fn context_data(&self) -> Option<ContextData> {
        match self {
            Self::MissingAampKey(_, data) => data.clone(),
            Self::InvalidByml(_, data) => Some(ContextData::Byml(data.clone())),
            Self::InvalidParameter(_, data) => Some(ContextData::Parameter(data.clone())),
            _ => None,
        }
    }
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
    canon.replace(".s", ".").into()
}

pub mod prelude {
    use cow_utils::CowUtils;
    pub(crate) use smartstring::alias::String;
    pub type String32 = roead::types::FixedSafeString<32>;
    pub type String64 = roead::types::FixedSafeString<64>;
    pub type String256 = roead::types::FixedSafeString<256>;
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
                    Self(ParameterIO {
                        param_root: crate::util::diff_plist(
                            &self.$field.param_root,
                            &other.$field.param_root,
                        ),
                        version:    self.$field.version,
                        data_type:  self.$field.data_type.clone(),
                    })
                }

                fn merge(&self, diff: &Self) -> Self {
                    Self(ParameterIO {
                        data_type:  self.$field.data_type.clone(),
                        version:    self.$field.version,
                        param_root: crate::util::merge_plist(
                            &self.$field.param_root,
                            &diff.$field.param_root,
                        ),
                    })
                }
            }
        };
    }

    impl Mergeable for roead::aamp::ParameterIO {
        fn diff(&self, other: &Self) -> Self {
            Self {
                data_type:  self.data_type.clone(),
                version:    self.version,
                param_root: crate::util::diff_plist(&self.param_root, &other.param_root),
            }
        }

        fn merge(&self, diff: &Self) -> Self {
            Self {
                data_type:  self.data_type.clone(),
                version:    self.version,
                param_root: crate::util::merge_plist(&self.param_root, &diff.param_root),
            }
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

    #[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
    pub enum Endian {
        #[serde(rename = "Switch")]
        Little,
        #[serde(rename = "Wii U")]
        Big,
    }

    impl std::fmt::Display for Endian {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Endian::Little => f.write_str("Little (Switch)"),
                Endian::Big => f.write_str("Big (Wii U)"),
            }
        }
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

    impl From<rstb::Endian> for Endian {
        fn from(endian: rstb::Endian) -> Self {
            match endian {
                rstb::Endian::Little => Self::Little,
                rstb::Endian::Big => Self::Big,
            }
        }
    }

    impl From<Endian> for rstb::Endian {
        fn from(endian: Endian) -> Self {
            match endian {
                Endian::Little => Self::Little,
                Endian::Big => Self::Big,
            }
        }
    }

    pub trait Resource
    where
        Self: std::marker::Sized,
    {
        fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self>;
        fn into_binary(self, endian: Endian) -> Vec<u8>;
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
        pub path:     Cow<'static, str>,
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
                path:     std::borrow::Cow::Borrowed($path),
                location: ResourceLocation::Content,
            };

            impl SingleResource for $type {
                fn path() -> &'static crate::prelude::ResourcePath {
                    &PATH
                }
            }
        };

        ($type:ty, $path:expr,aoc) => {
            static PATH: crate::prelude::ResourcePath = crate::prelude::ResourcePath {
                path:     std::borrow::Cow::Borrowed($path),
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
        roead::sarc::Sarc::new(
            roead::yaz0::decompress(
                std::fs::read(jstr!("test/Actor/Pack/{name}.sbactorpack")).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    pub fn test_mod_actorpack(name: &str) -> roead::sarc::Sarc<'static> {
        roead::sarc::Sarc::new(
            roead::yaz0::decompress(
                std::fs::read(jstr!("test/Actor/Pack/{name}_Mod.sbactorpack")).unwrap(),
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
