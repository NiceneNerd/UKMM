use std::borrow::Cow;
use crate::LOCALIZATION;

pub trait LocString {
    fn localize(&self) -> Cow<'static, str>;
}

impl LocString for &'static str {
    fn localize(&self) -> Cow<'static, str> {
        LOCALIZATION.read().get(&self)
    }
}