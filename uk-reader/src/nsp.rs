use crate::Result;
use std::path::Path;

#[derive(Debug)]
pub(crate) struct Nsp;

impl super::ROMReader for Nsp {
    fn get_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        todo!()
    }

    fn get_aoc_file_data(&self, name: impl AsRef<Path>) -> Result<Vec<u8>> {
        todo!()
    }

    fn file_exists(&self, name: impl AsRef<Path>) -> bool {
        todo!()
    }

    fn host_path(&self) -> &std::path::Path {
        todo!()
    }
}
