use thiserror::Error;

pub mod actor;

#[derive(Debug, Error)]
pub enum UKError {
    #[error("Parameter file missing key: {0}")]
    MissingAampKey(String),
    #[error("Wrong type for parameter value")]
    WrongAampType(#[from] roead::aamp::AampError),
}

pub type Result<T> = std::result::Result<T, UKError>;

#[cfg(test)]
mod tests {
    fn test_actorpack() -> roead::sarc::Sarc<'static> {
        println!("{}", std::env::current_dir().unwrap().display());
        roead::sarc::Sarc::read(
            roead::yaz0::decompress(std::fs::read("test/Enemy_Guardian_A.sbactorpack").unwrap())
                .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn it_works() {
        let actor = test_actorpack();
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AIProgram/Guardian_A.baiprog")
                .unwrap(),
        )
        .unwrap();
        let aiprog = crate::actor::aiprog::AIProgram::try_from(&pio).unwrap();
        serde_json::to_string(&aiprog).unwrap();
        aiprog.into_pio().to_binary();
    }
}
