#[derive(Debug)]
pub(crate) struct Unpacked;

impl super::ROMReader for Unpacked {
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
