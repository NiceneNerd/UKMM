#[derive(Debug)]
pub(crate) struct Unpacked;

impl super::RomReader for Unpacked {
    fn get_file(&self, name: &str) -> Option<super::ResourceData> {
        unimplemented!()
    }
}
