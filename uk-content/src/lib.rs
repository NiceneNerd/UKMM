use thiserror::Error;

pub mod actor;
pub mod util;

#[derive(Debug, Error)]
pub enum UKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(String),
    #[error("Wrong type for parameter value")]
    WrongAampType(#[from] roead::aamp::AampError),
}

pub type Result<T> = std::result::Result<T, UKError>;

pub mod prelude {
    pub trait IntoParameterIO {
        fn into_pio(self) -> roead::aamp::ParameterIO;
    }

    pub trait Mergeable {
        #[must_use]
        fn diff(&self, other: &Self) -> Self;
        #[must_use]
        fn merge(base: &Self, diff: &Self) -> Self;
    }
}

#[cfg(test)]
pub(crate) mod tests {
    pub fn test_actorpack() -> roead::sarc::Sarc<'static> {
        println!("{}", std::env::current_dir().unwrap().display());
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(std::fs::read("test/Enemy_Guardian_A.sbactorpack").unwrap())
                .unwrap(),
        )
        .unwrap()
    }
}
