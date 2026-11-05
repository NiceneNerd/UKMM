use std::collections::BTreeMap;

use itertools::Itertools;
use roead::byml::{map, Byml};
use serde::{Deserialize, Serialize};

use crate::{prelude::*, util::DeleteVec, Result, UKError};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]

pub enum StatusEffectValues {
    Special,
    Normal(DeleteVec<(i32, f32)>),
}

impl Default for StatusEffectValues {
    fn default() -> Self {
        Self::Normal(Default::default())
    }
}

impl TryFrom<&Byml> for StatusEffectValues {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let array = byml.as_array()?;
        if array
            .first()
            .ok_or(UKError::MissingBymlKey("Status effect list entry empty"))?
            .as_map()?
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
                    .as_map()?
                    .get("values")
                    .ok_or(UKError::MissingBymlKey(
                        "Status effect list entry missing values",
                    ))?
                    .as_array()?
                    .iter()
                    .enumerate()
                    .map(|(idx, val)| -> Result<(i32, f32)> {
                        Ok((
                            idx as i32,
                            val.as_map()?
                                .get("val")
                                .ok_or(UKError::MissingBymlKey(
                                    "Status effect list entry value missing val item",
                                ))?
                                .as_float()?,
                        ))
                    })
                    .collect::<Result<_>>()?,
            ))
        }
    }
}

impl From<StatusEffectValues> for Byml {
    fn from(val: StatusEffectValues) -> Self {
        match val {
            StatusEffectValues::Special => Byml::Array(vec![map!("special" => Byml::Bool(true))]),
            StatusEffectValues::Normal(values) => {
                Byml::Array(vec![
                    map!(
                        "special" => Byml::Bool(false)
                    ),
                    map!(
                        "values" => values
                            .into_iter()
                            .sorted_by(|a, b| Ord::cmp(&a.0, &b.0))
                            .map(|(_, v)| map!("val" => Byml::Float(v)))
                            .collect::<Byml>()
                    ),
                ])
            }
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

#[derive(Debug, Default, Clone, PartialEq, Deserialize, Serialize)]

pub struct StatusEffectList(pub BTreeMap<String, StatusEffectValues>);

impl TryFrom<&Byml> for StatusEffectList {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        Ok(Self(
            byml.as_array()?
                .first()
                .ok_or(UKError::MissingBymlKey("Status effect list missing root"))?
                .as_map()?
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
        Self::Array(vec![
            val.0
                .into_iter()
                .map(|(effect, values)| (effect.to_string(), values.into()))
                .collect::<Byml>(),
        ])
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
    fn from_binary(data: impl AsRef<[u8]>) -> Result<Self> {
        (&Byml::from_binary(data.as_ref())?).try_into()
    }

    fn into_binary(self, endian: Endian) -> Vec<u8> {
        Byml::from(self).to_binary(endian.into())
    }

    fn path_matches(path: impl AsRef<std::path::Path>) -> bool {
        path.as_ref().file_stem().and_then(|name| name.to_str()) == Some("StatusEffectList")
    }
}

#[allow(clippy::unwrap_used)]
#[cfg(test)]
mod tests {
    use roead::byml::Byml;

    use crate::prelude::*;

    fn load_status() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Ecosystem/StatusEffectList.sbyml").unwrap(),
            )
            .unwrap(),
        )
        .unwrap()
    }

    fn load_mod_status() -> Byml {
        Byml::from_binary(
            roead::yaz0::decompress(
                std::fs::read("test/Ecosystem/StatusEffectList.mod.sbyml").unwrap(),
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
        let byml2 = Byml::from_binary(data).unwrap();
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

    #[test]
    fn identify() {
        let path =
            std::path::Path::new("content/Pack/Bootup.pack//Ecosystem/StatusEffectList.sbyml");
        assert!(super::StatusEffectList::path_matches(path));
    }
}
