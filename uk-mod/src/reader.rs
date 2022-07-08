use crate::{Manifest, Meta};
use anyhow::{Context, Result, anyhow, bail};
use fs_err::File;
use std::{
    io::{BufReader, Read},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

type TarReader<'a> = Arc<Mutex<tar::Archive<zstd::Decoder<'a, BufReader<File>>>>>;

pub struct ModReader<'a> {
    path: PathBuf,
    meta: Meta,
    manifest: Manifest,
    tar: TarReader<'a>,
}

impl ModReader<'_> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut tar = tar::Archive::new(zstd::Decoder::new(File::open(&path)?)?);
        let mut manifest: Option<Manifest> = None;
        let mut meta: Option<Meta> = None;
        let mut buffer: Vec<u8> = vec![0; 10240];
        let mut iter = tar.entries()?.filter_map(Result::ok);
        while let Some(mut entry) = iter.next()
            && (meta.is_none() || manifest.is_none()) 
        {
            if entry.path()? == Path::new("manifest.yml") {
                let file_size = entry.size() as usize;
                buffer.resize(file_size, 0);
                let bytes_read = entry.read(&mut buffer)?;
                if bytes_read == file_size {
                    manifest = Some(
                        yaml_peg::serde::from_str::<Manifest>(std::str::from_utf8(&buffer)?)?
                            .swap_remove(0),
                    );
                } else {
                    bail!("Manifest file is corrupted, should be {} bytes, read {}", file_size, bytes_read);
                }
            } else if entry.path()? == Path::new("meta.toml") {
                let file_size = entry.size() as usize;
                buffer.resize(file_size as usize, 0);
                let bytes_read = entry.read(&mut buffer)?;
                if bytes_read == file_size as usize {
                    meta = Some(toml::from_slice(&buffer)?);
                } else {
                    bail!("Meta file is corrupted, should be {} bytes, read {}", file_size, bytes_read);
                }
            }
        }
        if let Some(manifest) = manifest && let Some(meta) = meta {
            Ok(Self {
                path,
                meta,
                manifest,
                tar: Arc::new(Mutex::new(tar)),
            })
        } else {
            Err(anyhow!("No manifest or meta found in mod"))
        }
    }
}

#[cfg(test)] 
mod tests {
    use super::*;
    #[test]
    fn open_mod() {
        let mod_reader = ModReader::open("test/wiiu.tar.zst").unwrap();
        dbg!(mod_reader.manifest);
        dbg!(mod_reader.meta);
    }
}
