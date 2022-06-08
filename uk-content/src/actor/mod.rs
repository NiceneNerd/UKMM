pub mod info;
pub mod params;
pub mod residents;

use roead::byml::Byml;

pub(crate) fn extract_info_param<T: TryFrom<roead::aamp::Parameter> + Into<Byml> + Clone>(
    obj: &roead::aamp::ParameterObject,
    key: &str,
) -> crate::Result<Option<Byml>> {
    Ok(obj
        .param(key)
        .map(|v| -> crate::Result<T> {
            v.clone()
                .try_into()
                .map_err(|_| crate::UKError::WrongAampType(roead::aamp::AampError::TypeError))
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
                .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v))),
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
                    v.and_then(|v| (!crate::actor::is_byml_null(&v)).then(|| (k.to_owned(), v)))
                }),
        );
    };
}

pub(crate) fn is_byml_null(byml: &Byml) -> bool {
    match byml {
        Byml::Float(v) => *v == 0.0,
        Byml::Int(v) => *v == 0,
        Byml::String(v) => v.is_empty(),
        _ => true,
    }
}

pub(crate) use info_params;
pub(crate) use info_params_filtered;

pub trait InfoSource {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()>;
}
