use anyhow::Context;
use roead::byml::Byml;
use serde::{Deserialize, Serialize};
use smartstring::{LazyCompact, SmartString};
#[cfg(feature = "ui")]
use uk_ui_derive::Editable;

use crate::{
    prelude::Mergeable,
    util::{bhash, DeleteVec},
    Result, UKError,
};

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct System {
    pub cei:    DeleteVec<Cei>,
    pub fa:     SmartString<LazyCompact>,
    pub falr:   i32,
    pub falrmr: f32,
    pub fca:    SmartString<LazyCompact>,
    pub lrmr:   f32,
    pub mea:    SmartString<LazyCompact>,
    pub nmmr:   DeleteVec<f32>,
    pub nmssr:  DeleteVec<i32>,
    pub sfalr:  i32,
    pub ssaet:  i32,
}

impl TryFrom<&Byml> for System {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            cei:    hash
                .get("CEI")
                .ok_or(UKError::MissingBymlKey("System missing CEI"))?
                .as_array()
                .map_err(|_| UKError::WrongBymlType("not an array".into(), "an array"))?
                .iter()
                .map(|b| Cei::try_from(b).context("Failed to parse CEI").unwrap())
                .collect(),
            fa:     hash
                .get("FA")
                .ok_or(UKError::MissingBymlKey("System missing FA"))?
                .as_string()
                .map_err(|_| UKError::WrongBymlType("not a string".into(), "a string"))
                .unwrap()
                .clone(),
            falr:   hash
                .get("FALR")
                .ok_or(UKError::MissingBymlKey("System missing FALR"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
            falrmr: hash
                .get("FALRMR")
                .ok_or(UKError::MissingBymlKey("System missing FALRMR"))?
                .as_float()
                .map_err(|_| UKError::WrongBymlType("not a float".into(), "a float"))
                .unwrap(),
            fca:    hash
                .get("FCA")
                .ok_or(UKError::MissingBymlKey("System missing FCA"))?
                .as_string()
                .map_err(|_| UKError::WrongBymlType("not a string".into(), "a string"))
                .unwrap()
                .clone(),
            lrmr:   hash
                .get("LRMR")
                .ok_or(UKError::MissingBymlKey("System missing LRMR"))?
                .as_float()
                .map_err(|_| UKError::WrongBymlType("not a float".into(), "a float"))
                .unwrap(),
            mea:    hash
                .get("MEA")
                .ok_or(UKError::MissingBymlKey("System missing MEA"))?
                .as_string()
                .map_err(|_| UKError::WrongBymlType("not a string".into(), "a string"))
                .unwrap()
                .clone(),
            nmmr:   hash
                .get("NMMR")
                .ok_or(UKError::MissingBymlKey("System missing NMMR"))?
                .as_array()
                .map_err(|_| UKError::WrongBymlType("not an array".into(), "an array"))?
                .iter()
                .map(|b| {
                    b.as_float()
                        .map_err(|_| UKError::WrongBymlType("not a float".into(), "a float"))
                        .unwrap()
                })
                .collect(),
            nmssr:  hash
                .get("NMSSR")
                .ok_or(UKError::MissingBymlKey("System missing NMSSR"))?
                .as_array()
                .map_err(|_| UKError::WrongBymlType("not an array".into(), "an array"))?
                .iter()
                .map(|b| {
                    b.as_i32()
                        .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                        .unwrap()
                })
                .collect(),
            sfalr:  hash
                .get("SFALR")
                .ok_or(UKError::MissingBymlKey("System missing SFALR"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
            ssaet:  hash
                .get("SSAET")
                .ok_or(UKError::MissingBymlKey("System missing SSAET"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
        })
    }
}

impl From<System> for Byml {
    fn from(val: System) -> Byml {
        bhash! {
            "CEI" => val.cei.iter().map(Byml::from).collect(),
            "FA" => val.fa.clone().into(),
            "FALR" => val.falr.into(),
            "FALRMR" => val.falrmr.into(),
            "FCA" => val.fca.clone().into(),
            "LRMR" => val.lrmr.into(),
            "MEA" => val.mea.clone().into(),
            "NMMR" => val.nmmr.iter().map(|n| Byml::Float(*n)).collect(),
            "NMSSR" => val.nmssr.iter().map(|n| Byml::I32(*n)).collect(),
            "SFALR" => val.sfalr.into(),
            "SSAET" => val.ssaet.into(),
        }
    }
}

impl Mergeable for System {
    fn diff(&self, other: &Self) -> Self {
        Self {
            cei:    self.cei.diff(&other.cei),
            fa:     other.fa.clone(),
            falr:   other.falr,
            falrmr: other.falrmr,
            fca:    other.fca.clone(),
            lrmr:   other.lrmr,
            mea:    other.mea.clone(),
            nmmr:   self.nmmr.diff(&other.nmmr),
            nmssr:  self.nmssr.diff(&other.nmssr),
            sfalr:  other.sfalr,
            ssaet:  other.ssaet,
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            cei:    self.cei.merge(&diff.cei),
            fa:     diff.fa.clone(),
            falr:   diff.falr,
            falrmr: diff.falrmr,
            fca:    diff.fca.clone(),
            lrmr:   diff.lrmr,
            mea:    diff.mea.clone(),
            nmmr:   self.nmmr.merge(&diff.nmmr),
            nmssr:  self.nmssr.merge(&diff.nmssr),
            sfalr:  diff.sfalr,
            ssaet:  diff.ssaet,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[cfg_attr(feature = "ui", derive(Editable))]
pub struct Cei {
    bt:  i32,
    mr:  f32,
    ma:  i32,
    mi:  i32,
    ssa: i32,
    t:   i32,
}

impl TryFrom<&Byml> for Cei {
    type Error = UKError;

    fn try_from(byml: &Byml) -> Result<Self> {
        let hash = byml.as_hash()?;
        Ok(Self {
            bt:  hash
                .get("BT")
                .ok_or(UKError::MissingBymlKey("CEI missing BT"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
            mr:  hash
                .get("MR")
                .ok_or(UKError::MissingBymlKey("CEI missing MR"))?
                .as_float()
                .map_err(|_| UKError::WrongBymlType("not a float".into(), "a float"))
                .unwrap(),
            ma:  hash
                .get("Ma")
                .ok_or(UKError::MissingBymlKey("CEI missing Ma"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
            mi:  hash
                .get("Mi")
                .ok_or(UKError::MissingBymlKey("CEI missing Mi"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
            ssa: hash
                .get("SSA")
                .ok_or(UKError::MissingBymlKey("CEI missing SSA"))?
                .as_i32()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
            t:   hash
                .get("T")
                .ok_or(UKError::MissingBymlKey("CEI missing T"))?
                .as_int::<i32>()
                .map_err(|_| UKError::WrongBymlType("not an integer".into(), "an integer"))
                .unwrap(),
        })
    }
}

impl From<&Cei> for Byml {
    fn from(val: &Cei) -> Byml {
        bhash! {
            "BT" => val.bt.into(),
            "MR" => val.mr.into(),
            "Ma" => val.ma.into(),
            "Mi" => val.mi.into(),
            "SSA" => val.ssa.into(),
            "T" => {
                if val.t < 0 {
                    Byml::U32(val.t as u32)
                }
                else {
                    Byml::I32(val.t)
                }
            },
        }
    }
}

impl Mergeable for Cei {
    fn diff(&self, other: &Self) -> Self {
        Self {
            bt:  other.bt,
            mr:  other.mr,
            ma:  other.ma,
            mi:  other.mi,
            ssa: other.ssa,
            t:   other.t,
        }
    }

    fn merge(&self, diff: &Self) -> Self {
        Self {
            bt:  diff.bt,
            mr:  diff.mr,
            ma:  diff.ma,
            mi:  diff.mi,
            ssa: diff.ssa,
            t:   diff.t,
        }
    }
}
