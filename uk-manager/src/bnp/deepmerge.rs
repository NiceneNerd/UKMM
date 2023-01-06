use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::{
    aamp::{ParameterIO, ParameterList, ParameterListing},
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use rustc_hash::FxHashMap;
use uk_content::util::merge_plist;

use super::BnpConverter;

type DiffMap = FxHashMap<String, DiffEntry>;

pub enum DiffEntry {
    Sarc(DiffMap),
    Aamp(ParameterList),
}

impl DiffEntry {
    pub fn as_sarc(&self) -> &DiffMap {
        if let DiffEntry::Sarc(ref map) = self {
            map
        } else {
            panic!("Not a SARC entry")
        }
    }

    pub fn as_mut_sarc(&mut self) -> &mut DiffMap {
        if let DiffEntry::Sarc(ref mut map) = self {
            map
        } else {
            panic!("Not a SARC entry")
        }
    }
}

fn handle_diff_entry(sarc: &mut SarcWriter, nest_root: &str, contents: &DiffEntry) -> Result<()> {
    let nested_bytes = sarc
        .get_file(nest_root)
        .with_context(|| format!("SARC missing file at {nest_root}"))?;
    match contents {
        DiffEntry::Sarc(nest_map) => {
            let mut nest_sarc = SarcWriter::from_sarc(&Sarc::new(nested_bytes)?);
            for (nested_file, nested_contents) in nest_map {
                handle_diff_entry(&mut nest_sarc, nested_file, nested_contents)?;
            }
            let data = nest_sarc.to_binary();
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
        DiffEntry::Aamp(plist) => {
            let mut pio = ParameterIO::from_binary(nested_bytes)?;
            pio.param_root = merge_plist(&pio.param_root, plist);
            let data = pio.to_binary();
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
    }
    Ok(())
}

impl BnpConverter {
    #[allow(irrefutable_let_patterns)]
    pub fn handle_deepmerge(&self) -> Result<()> {
        let deepmerge_path = self.path.join("logs/deepmerge.aamp");
        if deepmerge_path.exists() {
            let pio = ParameterIO::from_binary(fs::read(deepmerge_path)?)?;
            let diff = pio
                .object("FileTable")
                .context("Deepmerge log missing file table")?
                .0
                .values()
                .filter_map(|s| s.as_str().ok())
                .try_fold(
                    FxHashMap::default(),
                    |mut acc, file| -> Result<FxHashMap<String, DiffEntry>> {
                        let parts = file.split("//").collect::<Vec<_>>();
                        if parts.is_empty() {
                            anyhow::bail!("Why are there no diff path parts?");
                        }
                        let root_path = parts[0];
                        let root = acc
                            .entry(root_path.to_string())
                            .or_insert_with(|| DiffEntry::Sarc(Default::default()))
                            .as_mut_sarc();
                        let parent = if parts.len() == 3 {
                            root.entry(parts[1].into())
                                .or_insert_with(|| DiffEntry::Sarc(Default::default()))
                                .as_mut_sarc()
                        } else if parts.len() == 2 {
                            root
                        } else {
                            &mut acc
                        };
                        let plist = pio
                            .list(file)
                            .cloned()
                            .context("Missing entry in deepmerge log")?;
                        parent.insert(parts[parts.len() - 1].to_string(), DiffEntry::Aamp(plist));
                        Ok(acc)
                    },
                )?;
            diff.into_par_iter()
                .try_for_each(|(root, contents)| -> Result<()> {
                    let base_path = self.path.join(&root);
                    base_path.parent().iter().try_for_each(fs::create_dir_all)?;
                    match contents {
                        DiffEntry::Sarc(map) => {
                            let mut sarc =
                                self.open_or_create_sarc(&base_path, self.trim_prefixes(&root))?;
                            map.iter().try_for_each(|(nest_root, contents)| {
                                handle_diff_entry(&mut sarc, nest_root, contents)
                            })?;
                            fs::write(&base_path, compress_if(&sarc.to_binary(), &root))?;
                        }
                        DiffEntry::Aamp(plist) => {
                            let mut pio = ParameterIO::from_binary(
                                self.dump.get_bytes_uncached(self.trim_prefixes(&root))?,
                            )?;
                            pio.param_root = merge_plist(&pio.param_root, &plist);
                            fs::write(base_path, pio.to_binary())?;
                        }
                    }
                    Ok(())
                })?;
        }
        Ok(())
    }
}
