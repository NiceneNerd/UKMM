use crate::Result;
use fs_err as fs;
use std::{
    io::Cursor,
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Debug)]
pub(crate) struct Nsp {
    keys: cntx::key::Keyset,
}

impl Nsp {
    pub(crate) fn new(key_path: impl AsRef<Path>, nsp_path: impl AsRef<Path>) -> Result<Self> {
        let key_file = fs::File::open(key_path.as_ref())?;
        let keys = cntx::key::Keyset::from(&key_file)?;
        let nsp_file = fs::File::open(nsp_path.as_ref())?;
        let mut pfs0 = cntx::pfs0::PFS0::new(Arc::new(Mutex::new(nsp_file)))?;
        let files = pfs0.list_files()?;
        for (i, file) in files.into_iter().enumerate() {
            let size = pfs0.get_file_size(i)?;
            println!("{} is {} bytes", file, size);
            if file == "861707001401bbce36c7e421efac76d4.nca" {
                let nca_reader = pfs0.get_file_reader(i)?;
                let mut nca = cntx::nca::NCA::new(Arc::new(Mutex::new(nca_reader)), &keys, None)?;
                let fs_count = nca.get_filesystem_count();
                println!("NCA has {} filesystems", fs_count);
                for i in 0..fs_count {
                    if let Ok(mut romfs) = nca.open_romfs_filesystem(i) {
                        let mut dir_iterator = romfs.open_dir_iterator("".into())?;
                        let file_count = dir_iterator.get_file_count();
                        for _ in 0..file_count {
                            let (name, subsize) = dir_iterator.next_file()?;
                            println!("Subfile {} is {} bytes", name, subsize);
                        }
                    } else if let Ok(psf) = nca.open_pfs0_filesystem(i) {
                        println!("PFS0 has these files:\n{:?}", psf.list_files());
                    }
                }
            }
        }
        Ok(Self { keys })
    }
}

impl super::ResourceLoader for Nsp {
    fn get_data(&self, name: &Path) -> Result<Vec<u8>> {
        todo!()
    }

    fn get_aoc_file_data(&self, name: &Path) -> Result<Vec<u8>> {
        todo!()
    }

    fn file_exists(&self, name: &Path) -> bool {
        todo!()
    }

    fn host_path(&self) -> &std::path::Path {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn read_nsp() {
        let nsp = Nsp::new(
            "/data/GameDuo/NX/prod.keys",
            "/data/Downloads/The_Legend_of_Zelda_BotW/The Legend of Zelda Breath of the Wild.nsp",
        )
        .unwrap();
    }
}
