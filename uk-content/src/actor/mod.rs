pub mod info;
pub mod params;
pub mod residents;

pub trait InfoSource {
    fn update_info(&self, info: &mut roead::byml::Hash) -> crate::Result<()>;
}
