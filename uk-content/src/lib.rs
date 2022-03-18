use thiserror::Error;

pub mod actor;
pub mod constants;
pub mod util;

#[derive(Debug, Error)]
pub enum UKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(String),
    #[error("Wrong type for parameter value")]
    WrongAampType(#[from] roead::aamp::AampError),
    #[error("Invalid weather value: {0}")]
    InvalidWeather(#[from] strum::ParseError),
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

    pub trait Mergeable {
        #[must_use]
        fn diff(&self, other: &Self) -> Self;
        #[must_use]
        fn merge(base: &Self, diff: &Self) -> Self;
    }

    pub trait SimpleMergeableAamp {
        fn inner(&self) -> &roead::aamp::ParameterIO;
    }

    impl<'a, T: SimpleMergeableAamp + From<roead::aamp::ParameterIO>> Mergeable for T {
        fn diff(&self, other: &Self) -> Self {
            crate::util::diff_plist(self.inner(), other.inner()).into()
        }

        fn merge(base: &Self, diff: &Self) -> Self {
            crate::util::merge_plist(base.inner(), diff.inner()).into()
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    pub fn test_base_actorpack() -> roead::sarc::Sarc<'static> {
        println!("{}", std::env::current_dir().unwrap().display());
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(std::fs::read("test/Enemy_Guardian_A.sbactorpack").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    pub fn test_mod_actorpack() -> roead::sarc::Sarc<'static> {
        println!("{}", std::env::current_dir().unwrap().display());
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(
                std::fs::read("test/Enemy_Guardian_A_Mod.sbactorpack").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }
}
