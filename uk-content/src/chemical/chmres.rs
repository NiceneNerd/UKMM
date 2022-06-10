use crate::{prelude::*, util::DeleteMap, Result, UKError};
use join_str::jstr;
use roead::aamp::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ChemicalRes {
    pub world: DeleteMap<String, ParameterObject>,
    pub material: DeleteMap<String, ParameterObject>,
    pub element: DeleteMap<String, ParameterObject>,
}

impl TryFrom<&ParameterIO> for ChemicalRes {
    type Error = UKError;

    fn try_from(pio: &ParameterIO) -> Result<Self> {
        let parse_res = |key| -> Result<DeleteMap<String, ParameterObject>> {
            pio.list(key)
                .ok_or_else(|| UKError::MissingAampKeyD(jstr!("Chemical res missing {key}")))?
                .objects
                .0
                .values()
                .map(|obj| -> Result<(String, ParameterObject)> {
                    Ok((
                        obj.param("label")
                            .ok_or(UKError::MissingAampKey("Chemical res entry missing label"))?
                            .as_string()?
                            .to_owned(),
                        obj.clone(),
                    ))
                })
                .collect::<Result<_>>()
        };

        Ok(Self {
            world: parse_res("world")?,
            material: parse_res("material")?,
            element: parse_res("element")?,
        })
    }
}

impl From<ChemicalRes> for ParameterIO {
    fn from(val: ChemicalRes) -> Self {
        let gen_res = |res: DeleteMap<String, ParameterObject>| -> ParameterList {
            ParameterList::new().with_objects(
                res.into_iter()
                    .enumerate()
                    .map(|(i, (_, obj))| (lexical::to_string(i), obj)),
            )
        };
        Self::new()
            .with_list("world", gen_res(val.world))
            .with_list("material", gen_res(val.material))
            .with_list("element", gen_res(val.element))
    }
}

impl Mergeable for ChemicalRes {
    fn diff(&self, other: &Self) -> Self {
        Self {
            world: self.world.diff(&other.world),
            material: self.material.diff(&other.material),
            element: self.element.diff(&other.element),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            world: self.world.merge(&diff.world),
            material: self.material.merge(&diff.material),
            element: self.element.merge(&diff.element),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::aamp::ParameterIO;

    fn load_chmres() -> ParameterIO {
        ParameterIO::from_binary(&std::fs::read("test/Chemical/system.bchmres").unwrap()).unwrap()
    }

    fn load_mod_chmres() -> ParameterIO {
        ParameterIO::from_binary(&std::fs::read("test/Chemical/system.mod.bchmres").unwrap())
            .unwrap()
    }

    #[test]
    fn serde() {
        let pio = load_chmres();
        let chmres = super::ChemicalRes::try_from(&pio).unwrap();
        let data = ParameterIO::from(chmres.clone()).to_binary();
        let pio2 = ParameterIO::from_binary(&data).unwrap();
        let chmres2 = super::ChemicalRes::try_from(&pio2).unwrap();
        assert_eq!(chmres, chmres2);
    }

    #[test]
    fn diff() {
        let pio = load_chmres();
        let chmres = super::ChemicalRes::try_from(&pio).unwrap();
        let pio2 = load_mod_chmres();
        let chmres2 = super::ChemicalRes::try_from(&pio2).unwrap();
        let _diff = chmres.diff(&chmres2);
    }

    #[test]
    fn merge() {
        let pio = load_chmres();
        let chmres = super::ChemicalRes::try_from(&pio).unwrap();
        let pio2 = load_mod_chmres();
        let chmres2 = super::ChemicalRes::try_from(&pio2).unwrap();
        let diff = chmres.diff(&chmres2);
        let merged = chmres.merge(&diff);
        assert_eq!(merged, chmres2)
    }
}
