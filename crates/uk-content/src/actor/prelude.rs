use roead::{aamp::*, byml::Byml};

use crate::Result;

pub(crate) fn extract_info_param<T: TryFrom<Parameter> + Into<Byml> + Clone>(
    obj: &ParameterObject,
    key: &str,
) -> Result<Option<Byml>> {
    Ok(obj
        .get(key)
        .map(|v| -> Result<T> {
            v.clone()
                .try_into()
                .map_err(|_| crate::UKError::OtherD(format!("Wrong AAMP type for {:?}", v)))
        })
        .transpose()?
        .map(|v| v.into()))
}

macro_rules! info_params {
    (
        $o: expr,
        $i: expr,
        {
            $(($k: expr, $v: expr, $t: ty)),* $(,)?
        }
    ) => {
        $i.extend(
            [
                $(
                    ($k, crate::actor::extract_info_param::<$t>($o, $v)?),
                )*
            ]
                .into_iter()
                .filter_map(|(k, v)| v.map(|v| (k.into(), v))),
        );
    };
}

macro_rules! info_params_filtered {
    (
        $o: expr,
        $i: expr,
        {
            $(($k: expr, $v: expr, $t: ty)),* $(,)?
        }
    ) => {
        $i.extend(
            [
                $(
                    ($k, crate::actor::extract_info_param::<$t>($o, $v)?),
                )*
            ]
                .into_iter()
                .filter_map(|(k, v)| {
                    v.and_then(|v| (!crate::actor::is_byml_null(&v)).then(|| (k.into(), v)))
                }),
        );
    };
}

pub(crate) fn is_byml_null(byml: &Byml) -> bool {
    match byml {
        Byml::Float(v) => *v == 0.0,
        Byml::I32(v) => *v == 0,
        Byml::String(v) => v.is_empty(),
        _ => true,
    }
}

pub(crate) use info_params;
pub(crate) use info_params_filtered;

use crate::prelude::Resource;

pub trait InfoSource {
    fn update_info(&self, info: &mut roead::byml::Map) -> Result<()>;
}

pub trait ParameterResource: Resource {
    fn path(name: &str) -> String;
}
