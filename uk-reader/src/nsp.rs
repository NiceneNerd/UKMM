#[derive(Debug)]
pub(crate) struct Nsp;

impl super::ROMReader for Nsp {
    fn get_file_data(&self, name: &str) -> Option<super::ResourceData> {
        unimplemented!()
    }

    fn get_aoc_file_data(&self, name: &str) -> Option<super::ResourceData> {
        unimplemented!()
    }

    fn file_exists(&self, name: &str) -> bool {
        unimplemented!()
    }
}
