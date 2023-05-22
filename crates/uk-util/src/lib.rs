/// Temporary type until feature lazy_cell is stabilized
pub type Lazy<T> = once_cell::sync::Lazy<T>;

pub trait OptionResultExt {
    type T;
    fn contains<U>(&self, other: &U) -> bool
    where
        Self::T: PartialEq<U>;
    fn inspect<F: FnOnce(&Self::T)>(self, f: F) -> Self;
}

impl<T> OptionResultExt for Option<T> {
    type T = T;

    #[inline(always)]
    fn contains<U>(&self, other: &U) -> bool
    where
        Self::T: PartialEq<U>,
    {
        match self {
            Some(inner) => inner.eq(other),
            None => false,
        }
    }

    fn inspect<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Some(ref t) = self {
            f(t);
        }

        self
    }
}

impl<T, E> OptionResultExt for Result<T, E> {
    type T = T;

    #[inline(always)]
    fn contains<U>(&self, other: &U) -> bool
    where
        Self::T: PartialEq<U>,
    {
        match self {
            Ok(inner) => inner == other,
            Err(_) => false,
        }
    }

    fn inspect<F: FnOnce(&T)>(self, f: F) -> Self {
        if let Ok(ref t) = self {
            f(t);
        }

        self
    }
}

pub trait OptionExt<T: Default> {
    fn get_or_insert_default(&mut self) -> &mut T;
}

impl<T: Default> OptionExt<T> for Option<T> {
    #[inline(always)]
    fn get_or_insert_default(&mut self) -> &mut T {
        self.get_or_insert_with(T::default)
    }
}

pub trait PathExt
where
    Self: Sized,
{
    fn exists_then(self) -> Option<Self>;
}

impl PathExt for &std::path::Path {
    #[inline(always)]
    fn exists_then(self) -> Option<Self> {
        self.exists().then_some(self)
    }
}

impl PathExt for std::path::PathBuf {
    #[inline(always)]
    fn exists_then(self) -> Option<Self> {
        self.exists().then_some(self)
    }
}
