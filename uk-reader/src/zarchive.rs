#[derive(Debug)]
pub(crate) struct ZArchive;

impl super::RomReader for ZArchive {
    fn get_file(&self, name: &str) -> Option<super::ResourceData> {
        unimplemented!()
    }
}
