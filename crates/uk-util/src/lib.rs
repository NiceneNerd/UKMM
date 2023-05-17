pub trait OptionResultExt {
    type T;
    fn contains<U>(&self, other: &U) -> bool
    where
        Self::T: PartialEq<U>;
}

impl<T> OptionResultExt for Option<T> {
    type T = T;

    fn contains<U>(&self, other: &U) -> bool
    where
        Self::T: PartialEq<U>,
    {
        match self {
            Some(inner) => inner.eq(other),
            None => false,
        }
    }
}

impl<T, E> OptionResultExt for Result<T, E> {
    type T = T;

    fn contains<U>(&self, other: &U) -> bool
    where
        Self::T: PartialEq<U>,
    {
        match self {
            Ok(inner) => inner == other,
            Err(_) => false,
        }
    }
}

pub trait OptionExt<T: Default> {
    fn get_or_insert_default(&mut self) -> &mut T;
}

impl<T: Default> OptionExt<T> for Option<T> {
    fn get_or_insert_default(&mut self) -> &mut T {
        self.get_or_insert_with(T::default)
    }
}
