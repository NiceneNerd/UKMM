use std::path::Path;

use crate::Result;

#[derive(Debug)]
pub(crate) struct Unpacked;

impl super::ROMReader for Unpacked {
    fn get_file_data(&self, name: impl AsRef<Path>) -> Result<super::ResourceData> {
        todo!()
    }

    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> Result<super::ResourceData> {
        todo!()
    }

    fn file_exists(&self, name: impl AsRef<Path>) -> bool {
        todo!()
    }

    fn host_path(&self) -> &std::path::Path {
        todo!()
    }
}
