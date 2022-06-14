#![feature(let_chains, type_alias_impl_trait, drain_filter)]
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
pub mod quest;
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
    #[error("Invalid weather value: {0}")]
    InvalidWeatherOrTime(String),
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

pub mod prelude {
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

    pub(crate) use impl_simple_byml;

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

    pub trait Resource
    where
        Self: std::marker::Sized,
    {
        fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self>;
        fn into_binary(self, endian: Endian) -> Vec<u8>;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use join_str::jstr;

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
}
