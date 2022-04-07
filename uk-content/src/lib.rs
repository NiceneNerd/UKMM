#![feature(let_chains)]
use thiserror::Error;

pub mod actor;
pub mod constants;
pub mod util;

#[derive(Debug, Error)]
pub enum UKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(&'static str),
    #[error("Parameter file missing key: {0}")]
    MissingAampKeyD(String),
    #[error("Wrong type for parameter value")]
    WrongAampType(#[from] roead::aamp::AampError),
    #[error("Invalid weather value: {0}")]
    InvalidWeatherOrTime(String),
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

    impl<'a, T> Mergeable<roead::aamp::ParameterIO> for T
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

    impl<'a, T> Mergeable<roead::byml::Byml> for T
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
    pub fn test_base_actorpack(name: &str) -> roead::sarc::Sarc<'static> {
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(
                std::fs::read(&format!("test/Actor/{}.sbactorpack", name)).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    pub fn test_mod_actorpack(name: &str) -> roead::sarc::Sarc<'static> {
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(
                std::fs::read(&format!("test/Actor/{}_Mod.sbactorpack", name)).unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }
}
