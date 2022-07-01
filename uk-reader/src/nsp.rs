#[derive(Debug)]
pub(crate) struct Nsp;

impl super::RomReader for Nsp {
    fn get_file(&self, name: &str) -> Option<super::ResourceData> {
        unimplemented!()
    }
}
