use crate::{
    actor::ParameterResource,
    prelude::*,
    util::{self, DeleteVec},
    Result, UKError,
};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AttClient {
    pub client_params: ParameterObject,
    pub checks: DeleteVec<ParameterList>,
}

impl TryFrom<&ParameterIO> for AttClient {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        Ok(Self {
            client_params: pio
                .object("AttClientParams")
                .ok_or(UKError::MissingAampKey("Attention client missing params"))?
                .clone(),
            checks: pio.lists.0.values().cloned().collect(),
        })
    }
}

impl From<AttClient> for ParameterIO {
    fn from(val: AttClient) -> Self {
        ParameterIO::new()
            .with_object("AttClientParams", val.client_params)
            .with_lists(
                val.checks
                    .into_iter()
                    .enumerate()
                    .map(|(i, check)| (jstr!("Check_{&lexical::to_string(i)}"), check)),
            )
    }
}

impl Mergeable for AttClient {
    fn diff(&self, other: &Self) -> Self {
        Self {
            client_params: util::diff_pobj(&self.client_params, &other.client_params),
            checks: self.checks.diff(&other.checks),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            client_params: util::merge_pobj(&self.client_params, &diff.client_params),
            checks: self.checks.merge(&diff.checks),
        }
    }
}

impl ParameterResource for AttClient {
    fn path(name: &str) -> String {
        jstr!("Actor/AttClient/{name}.batcl")
    }
}

impl Resource for AttClient {
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&ParameterIO::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, _endian: Endian) -> roead::Bytes {
        ParameterIO::from(self).to_binary()
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().extension().and_then(|ext| ext.to_str()) == Some("batcl")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;

    #[test]
    fn serde() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AttClient/Enemy_Guardian_LockOn.batcl")
                .unwrap(),
        )
        .unwrap();
        let atcl = super::AttClient::try_from(&pio).unwrap();
        let data = roead::aamp::ParameterIO::from(atcl.clone()).to_binary();
        let pio2 = roead::aamp::ParameterIO::from_binary(&data).unwrap();
        let atcl2 = super::AttClient::try_from(&pio2).unwrap();
        assert_eq!(atcl, atcl2);
    }

    #[test]
    fn diff() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AttClient/Enemy_Guardian_LockOn.batcl")
                .unwrap(),
        )
        .unwrap();
        let atcl = super::AttClient::try_from(&pio).unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/AttClient/Enemy_Guardian_LockOn.batcl")
                .unwrap(),
        )
        .unwrap();
        let atcl2 = super::AttClient::try_from(&pio2).unwrap();
        let diff = atcl.diff(&atcl2);
        println!("{}", serde_json::to_string_pretty(&diff).unwrap());
    }

    #[test]
    fn merge() {
        let actor = crate::tests::test_base_actorpack("Enemy_Guardian_A");
        let pio = roead::aamp::ParameterIO::from_binary(
            actor
                .get_file_data("Actor/AttClient/Enemy_Guardian_LockOn.batcl")
                .unwrap(),
        )
        .unwrap();
        let actor2 = crate::tests::test_mod_actorpack("Enemy_Guardian_A");
        let atcl = super::AttClient::try_from(&pio).unwrap();
        let pio2 = roead::aamp::ParameterIO::from_binary(
            actor2
                .get_file_data("Actor/AttClient/Enemy_Guardian_LockOn.batcl")
                .unwrap(),
        )
        .unwrap();
        let atcl2 = super::AttClient::try_from(&pio2).unwrap();
        let diff = atcl.diff(&atcl2);
        let merged = atcl.merge(&diff);
        assert_eq!(atcl2, merged);
    }

    #[test]
    fn identify() {
        let path = std::path::Path::new(
            "content/Actor/Pack/Enemy_Guardian_A.sbactorpack//Actor/AttClient/Enemy_Guardian_LockOn.batcl",
        );
        assert!(super::AttClient::path_matches(path));
    }
}
