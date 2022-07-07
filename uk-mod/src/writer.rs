use anyhow::{Context, Result};
use fs_err as fs;
use join_str::jstr;
use jwalk::WalkDir;
use path_slash::PathExt;
use rayon::prelude::*;
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uk_content::{
    canonicalize,
    prelude::Endian,
    resource::{ResourceData, ResourceRegister},
};

pub type TarWriter<'a> = Arc<Mutex<tar::Builder<zstd::Encoder<'a, fs::File>>>>;

pub struct ModBuilder<'a> {
    source_dir: PathBuf,
    dest_file: PathBuf,
    endian: Endian,
    tar: TarWriter<'a>,
    built_resources: Arc<Mutex<BTreeSet<String>>>,
}

impl ModBuilder<'_> {
    pub fn new(source: impl AsRef<Path>, dest: impl AsRef<Path>, endian: Endian) -> Result<Self> {
        let source_dir = source.as_ref().to_path_buf();
        if !source_dir.exists() {
            anyhow::bail!("Source directory does not exist: {}", source_dir.display());
        }
        let dest_file = dest.as_ref().to_path_buf();
        if dest_file.exists() {
            fs::remove_file(&dest_file)?;
        }
        let tar = Arc::new(Mutex::new(tar::Builder::new(zstd::Encoder::new(
            fs::File::create(&dest_file)?,
            3,
        )?)));
        Ok(Self {
            source_dir,
            dest_file,
            endian,
            tar,
            built_resources: Arc::new(Mutex::new(BTreeSet::new())),
        })
    }

    fn collect_resources(&self, dir: impl AsRef<Path>) -> Result<BTreeSet<String>> {
        let mut files = BTreeSet::new();
        for entry in WalkDir::new(dir.as_ref())
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let name = path
                .strip_prefix(&self.source_dir)
                .unwrap()
                .to_slash_lossy()
                .to_string();
            let canon = canonicalize(&name);
            let resource = ResourceData::from_binary(&name, std::fs::read(&path)?, todo!())
                .with_context(|| jstr!("Error parsing resource at {&name}"))?;
            self.tar.lock().unwrap().append_data(
                &mut tar::Header::new_gnu(),
                &canon,
                &*resource.to_binary(self.endian, todo!())?,
            )?;
            self.built_resources.lock().unwrap().insert(canon);
            files.insert(name);
        }
        Ok(files)
    }

    pub fn build(self) -> Result<()> {
        Ok(())
    }
}
