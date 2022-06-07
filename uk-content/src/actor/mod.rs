pub mod info;
pub mod params;
pub mod residents;

pub(crate) fn extract_info_param<
    T: TryFrom<roead::aamp::Parameter> + Into<roead::byml::Byml> + Clone,
>(
    obj: &roead::aamp::ParameterObject,
    key: &str,
) -> crate::Result<Option<roead::byml::Byml>> {
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

pub(crate) use info_params;

pub trait InfoSource {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()>;
}
