use crate::{prelude::*, util::DeleteVec, Result, UKError};
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum StatusEffectValues {
    Special,
    Normal(DeleteVec<f32>),
}

impl TryFrom<&Byml> for StatusEffectValues {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let array = byml.as_array()?;
        if array
            .get(0)
            .ok_or(UKError::MissingBymlKey("Status effect list entry empty"))?
            .as_hash()?
            .get("special")
            .ok_or(UKError::MissingBymlKey(
                "Status effect list entry missing special flag",
            ))?
            .as_bool()?
        {
            Ok(Self::Special)
        } else {
            Ok(Self::Normal(
                array
                    .get(1)
                    .ok_or(UKError::MissingBymlKey(
                        "Status effect list entry missing values",
                    ))?
                    .as_hash()?
                    .get("values")
                    .ok_or(UKError::MissingBymlKey(
                        "Status effect list entry missing values",
                    ))?
                    .as_array()?
                    .iter()
                    .map(|val| -> Result<f32> {
                        Ok(val
                            .as_hash()?
                            .get("val")
                            .ok_or(UKError::MissingBymlKey(
                                "Status effect list entry value missing val item",
                            ))?
                            .as_float()?)
                    })
                    .collect::<Result<_>>()?,
            ))
        }
    }
}

impl From<StatusEffectValues> for Byml {
    fn from(val: StatusEffectValues) -> Self {
        match val {
            StatusEffectValues::Special => {
                Self::Array(vec![[("special", Byml::Bool(true))].into_iter().collect()])
            }
            StatusEffectValues::Normal(values) => [
                [("special", Byml::Bool(false))]
                    .into_iter()
                    .collect::<Byml>(),
                [(
                    "values",
                    values
                        .into_iter()
                        .map(|v| [("val", Byml::Float(v))].into_iter().collect::<Byml>())
                        .collect::<Byml>(),
                )]
                .into_iter()
                .collect::<Byml>(),
            ]
            .into_iter()
            .collect::<Byml>(),
        }
    }
}

impl Mergeable for StatusEffectValues {
    fn diff(&self, other: &Self) -> Self {
        match (self, other) {
            (Self::Special, Self::Special) => Self::Special,
            (Self::Normal(self_values), Self::Normal(other_values)) => {
                Self::Normal(self_values.diff(other_values))
            }
            _ => panic!("Attempted to diff incompatible status effect types"),
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        match (self, diff) {
            (Self::Special, Self::Special) => Self::Special,
            (Self::Normal(self_values), Self::Normal(diff_values)) => {
                Self::Normal(self_values.merge(diff_values))
            }
            _ => panic!("Attempted to merge incompatible status effect types"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct StatusEffectList(pub BTreeMap<String, StatusEffectValues>);

impl TryFrom<&Byml> for StatusEffectList {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .get(0)
                .ok_or(UKError::MissingBymlKey("Status effect list missing root"))?
                .as_hash()?
                .iter()
                .map(|(effect, values)| -> Result<(String, StatusEffectValues)> {
                    Ok((effect.clone(), values.try_into()?))
                })
                .collect::<Result<_>>()?,
        ))
    }
}

impl From<StatusEffectList> for Byml {
    fn from(val: StatusEffectList) -> Self {
        Self::Array(vec![val
            .0
            .into_iter()
            .map(|(effect, values)| (effect, values.into()))
            .collect::<Byml>()])
    }
}

impl Mergeable for StatusEffectList {
    fn diff(&self, other: &Self) -> Self {
        Self(
            self.0
                .iter()
                .filter_map(|(effect, self_values)| {
                    let other_values = &other.0[effect];
                    (self_values != other_values)
                        .then(|| (effect.clone(), self_values.diff(other_values)))
                })
                .collect(),
        )
    }

    fn merge(&self, diff: &Self) -> Self {
        Self(
            self.0
                .iter()
                .map(|(effect, self_values)| {
                    (
                        effect.clone(),
                        diff.0
                            .get(effect)
                            .map(|diff_values| self_values.merge(diff_values))
                            .unwrap_or_else(|| self_values.clone()),
                    )
                })
                .collect(),
        )
    }
}

impl Resource for StatusEffectList {
    fn from_binary(data: impl AsRef<[u8]>) -> crate::Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: crate::prelude::Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("StatusEffectList")
    }
}

#[cfg(test)]
mod tests {
    use crate::prelude::*;
    use roead::byml::Byml;

    fn load_status() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Ecosystem/StatusEffectList.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_status() -> Byml {
        Byml::from_binary(
            &roead::yaz0::decompress(
                &std::fs::read("test/Ecosystem/StatusEffectList.mod.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn serde() {
        let byml = load_status();
        let status = super::StatusEffectList::try_from(&byml).unwrap();
        let data = Byml::from(status.clone()).to_binary(roead::Endian::Big);
        let byml2 = Byml::from_binary(&data).unwrap();
        let status2 = super::StatusEffectList::try_from(&byml2).unwrap();
        assert_eq!(status, status2);
    }

    #[test]
    fn diff() {
        let byml = load_status();
        let status = super::StatusEffectList::try_from(&byml).unwrap();
        let byml2 = load_mod_status();
        let status2 = super::StatusEffectList::try_from(&byml2).unwrap();
        let _diff = status.diff(&status2);
    }

    #[test]
    fn merge() {
        let byml = load_status();
        let status = super::StatusEffectList::try_from(&byml).unwrap();
        let byml2 = load_mod_status();
        let status2 = super::StatusEffectList::try_from(&byml2).unwrap();
        let diff = status.diff(&status2);
        let merged = status.merge(&diff);
        assert_eq!(merged, status2);
    }
}
