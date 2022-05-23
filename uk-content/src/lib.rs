#![feature(let_chains)]
#![feature(type_alias_impl_trait)]
use thiserror::Error;

pub mod actor;
pub mod chemical;
pub mod constants;
pub mod cooking;
pub mod data;
pub mod util;

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
    #[error("Invalid weather value: {0}")]
    InvalidWeatherOrTime(String),
    #[error("{0}")]
    Other(&'static str),
    #[error("{0}")]
    OtherD(String),
}

pub type Result<T> = std::result::Result<T, UKError>;

pub mod prelude {
    pub trait IntoParameterIO {
        fn into_pio(self) -> roead::aamp::ParameterIO;
    }

    impl<T: Into<roead::aamp::ParameterIO>> IntoParameterIO for T {
        fn into_pio(self) -> roead::aamp::ParameterIO {
            self.into()
        }
    }

    pub trait Convertible<T>: TryFrom<T> + Into<T> {}

    impl<T: TryFrom<T> + Into<T>> Convertible<T> for T {}

    pub trait Mergeable<T> {
        #[must_use]
        fn diff(&self, other: &Self) -> Self;
        #[must_use]
        fn merge(&self, diff: &Self) -> Self;
    }

    pub trait SimpleMergeableAamp {
        fn inner(&self) -> &roead::aamp::ParameterIO;
    }

    impl<T> Mergeable<roead::aamp::ParameterIO> for T
    where
        T: SimpleMergeableAamp
            + Convertible<roead::aamp::ParameterIO>
            + From<roead::aamp::ParameterIO>,
    {
        fn diff(&self, other: &Self) -> Self {
            crate::util::diff_plist(self.inner(), other.inner()).into()
        }

        fn merge(&self, diff: &Self) -> Self {
            crate::util::merge_plist(self.inner(), diff.inner()).into()
        }
    }

    pub trait ShallowMergeableByml {
        fn inner(&self) -> &roead::byml::Byml;
    }

    impl<T> Mergeable<roead::byml::Byml> for T
    where
        T: ShallowMergeableByml + Convertible<roead::byml::Byml> + From<roead::byml::Byml>,
    {
        fn diff(&self, other: &Self) -> Self {
            crate::util::diff_byml_shallow(self.inner(), other.inner()).into()
        }

        fn merge(&self, diff: &Self) -> Self {
            crate::util::merge_byml_shallow(self.inner(), diff.inner()).into()
        }
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
