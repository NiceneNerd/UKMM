use anyhow::Context;
use roead::byml::Byml;

use crate::UKError;

use super::DeleteMap;

fn warn_vecf_not_float<T>(val: T) where T: std::fmt::Debug {
    log::warn!(
        "Invalid value in Vectorf: {} {val:?}. Coercing to float...",
        std::any::type_name::<T>()
    )
}

pub(crate) fn try_get_vecf(value: &Byml) -> crate::Result<DeleteMap<char, f32>> {
    value.as_map()
        .context("Invalid Vectorf")?
        .iter()
        .enumerate()
        .map(|(i, (k, val))| {
            let maybe_key = k.chars().next();
            let key = maybe_key.ok_or(
                UKError::InvalidByml("Empty or invalid key".into(), value.clone())
            )?;
            match (key, val) {
                ('W'..='Z', Byml::Float(v)) => Ok((key, *v)),
                ('W'..='Z', Byml::I32(v)) => {
                    warn_vecf_not_float(v);
                    Ok((key, *v as f32))
                },
                ('W'..='Z', Byml::U32(v)) => {
                    warn_vecf_not_float(v);
                    Ok((key, *v as f32))
                },
                ('W'..='Z', Byml::I64(v)) => {
                    warn_vecf_not_float(v);
                    Ok((key, *v as f32))
                },
                ('W'..='Z', Byml::U64(v)) => {
                    warn_vecf_not_float(v);
                    Ok((key, *v as f32))
                },
                ('W'..='Z', Byml::Double(v)) => {
                    warn_vecf_not_float(v);
                    Ok((key, *v as f32))
                },
                ('W'..='Z', _) => Err(UKError::InvalidByml(format!("Invalid value for key {key}").into(), value.clone())),
                (_, Byml::Float(v)) => Err(UKError::InvalidByml(format!("Invalid key for value {v}").into(), value.clone())),
                (_, Byml::I32(v)) => {
                    warn_vecf_not_float(v);
                    Err(UKError::InvalidByml(format!("Invalid key for value {v}").into(), value.clone()))
                },
                (_, Byml::U32(v)) => {
                    warn_vecf_not_float(v);
                    Err(UKError::InvalidByml(format!("Invalid key for value {v}").into(), value.clone()))
                },
                (_, Byml::I64(v)) => {
                    warn_vecf_not_float(v);
                    Err(UKError::InvalidByml(format!("Invalid key for value {v}").into(), value.clone()))
                },
                (_, Byml::U64(v)) => {
                    warn_vecf_not_float(v);
                    Err(UKError::InvalidByml(format!("Invalid key for value {v}").into(), value.clone()))
                },
                (_, Byml::Double(v)) => {
                    warn_vecf_not_float(v);
                    Err(UKError::InvalidByml(format!("Invalid key for value {v}").into(), value.clone()))
                },
                _ => Err(UKError::InvalidByml(format!("Invalid index {i}").into(), value.clone())),
            }
        })
        .collect::<Result<DeleteMap<_, _>, _>>()
}
