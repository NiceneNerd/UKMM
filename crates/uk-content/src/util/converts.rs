use roead::byml::Byml;

pub trait FromByml
where
    Self: Sized,
{
    fn from_byml(byml: &Byml) -> crate::Result<Self>;
}

impl<T, E> FromByml for T
where
    T: for<'a> TryFrom<&'a Byml, Error = E>,
    crate::UKError: From<E>,
{
    #[inline(always)]
    fn from_byml(byml: &Byml) -> crate::Result<Self> {
        Ok(Self::try_from(byml)?)
    }
}
