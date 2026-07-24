use anyhow_ext::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::{
    aamp::ParameterIO,
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use uk_content::{
    prelude::{Mergeable, Resource},
    resource::ASList,
};

use super::{parse_aamp_diff, AampDiffEntry, BnpConverter};

fn handle_diff_entry(
    sarc: &mut SarcWriter,
    nest_root: &str,
    contents: &AampDiffEntry,
) -> Result<()> {
    let nested_bytes = match sarc.get_file(nest_root) {
        Some(b) => b,
        None => {
            log::warn!(
                "[lenient] SARC missing file at {nest_root}. \
                 Файл пропущен при конвертации BNP. Мод может вызвать баги в игре."
            );
            return Ok(());
        }
    };
    match contents {
        AampDiffEntry::Sarc(nest_map) => {
            let mut nest_sarc = SarcWriter::from_sarc(&Sarc::new(nested_bytes)?);
            for (nested_file, nested_contents) in nest_map {
                handle_diff_entry(&mut nest_sarc, nested_file, nested_contents)
                    .with_context(|| format!("Failed to process {}", nested_file))?;
            }
            let data = nest_sarc.to_binary();
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
        AampDiffEntry::Aamp(plist) => {
            let pio = ASList::try_from(&ParameterIO::from_binary(nested_bytes)?)?;
            let diff = ASList::try_from(&ParameterIO::new().with_root(plist.clone()))?;
            let data = pio
                .merge(&diff)
                .into_binary(uk_content::prelude::Endian::Little);
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
    }
    Ok(())
}

impl BnpConverter {
    pub fn handle_aslist(&self) -> Result<()> {
        let aslist_path = self.current_root.join("logs/aslist.aamp");
        if aslist_path.exists() {
            log::debug!("Processing AS list log");
            let pio = ParameterIO::from_binary(fs::read(aslist_path)?)?;
            let diff = parse_aamp_diff("FileTable", &pio)?;
            diff.into_par_iter()
                .for_each(|(root, contents)| {
                    let base_path = self.current_root.join(&root);
                    if let Err(e) = base_path.parent().iter().try_for_each(fs::create_dir_all) {
                        log::warn!("[lenient] Failed to create dir for {}: {}", base_path.display(), e);
                        return;
                    }
                    match contents {
                        AampDiffEntry::Sarc(map) => {
                            let mut sarc = match self.open_or_create_sarc(&base_path, self.trim_prefixes(&root)) {
                                Ok(s) => s,
                                Err(e) => {
                                    log::warn!(
                                        "[lenient] Failed to open or create SARC at {}: {}",
                                        base_path.display(), e
                                    );
                                    return;
                                }
                            };
                            map.iter().for_each(|(nest_root, contents)| {
                                if let Err(e) = handle_diff_entry(&mut sarc, nest_root, contents) {
                                    log::warn!(
                                        "[lenient] Failed to process {} in {}: {}. Пропущен.",
                                        nest_root, root, e
                                    );
                                }
                            });
                            if let Err(e) = fs::write(&base_path, compress_if(&sarc.to_binary(), &root)) {
                                log::warn!("[lenient] Failed to write {}: {}", base_path.display(), e);
                            }
                        }
                        AampDiffEntry::Aamp(plist) => {
                            let master_bytes = match self.get_master_bytes(self.trim_prefixes(&root)) {
                                Ok(b) => b,
                                Err(e) => {
                                    log::warn!("[lenient] Failed to get master bytes for {}: {}", root, e);
                                    return;
                                }
                            };
                            let pio = match ParameterIO::from_binary(master_bytes) {
                                Ok(p) => p,
                                Err(e) => {
                                    log::warn!("[lenient] Failed to parse AAMP for {}: {}", root, e);
                                    return;
                                }
                            };
                            let pio_aslist = match ASList::try_from(&pio) {
                                Ok(a) => a,
                                Err(e) => {
                                    log::warn!("[lenient] Failed to parse ASList for {}: {}", root, e);
                                    return;
                                }
                            };
                            let diff_pio = ParameterIO::new().with_root(plist);
                            let diff = match ASList::try_from(&diff_pio) {
                                Ok(a) => a,
                                Err(e) => {
                                    log::warn!("[lenient] Failed to parse diff ASList for {}: {}", root, e);
                                    return;
                                }
                            };
                            let data = pio_aslist
                                .merge(&diff)
                                .into_binary(uk_content::prelude::Endian::Little);
                            let data = compress_if(&data, &root);
                            if let Err(e) = fs::write(&base_path, data) {
                                log::warn!("[lenient] Failed to write {}: {}", base_path.display(), e);
                            }
                        }
                    }
                });
        }
        Ok(())
    }
}
