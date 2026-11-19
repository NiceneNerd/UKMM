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

use super::{AampDiffEntry, BnpConverter, parse_aamp_diff};

fn handle_diff_entry(
    sarc: &mut SarcWriter,
    nest_root: &str,
    contents: &AampDiffEntry,
) -> Result<()> {
    let nested_bytes = sarc
        .get_file(nest_root)
        .with_context(|| format!("SARC missing file at {nest_root}"))?;
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
                .try_for_each(|(root, contents)| -> Result<()> {
                    let base_path = self.current_root.join(&root);
                    base_path.parent().iter().try_for_each(fs::create_dir_all)?;
                    match contents {
                        AampDiffEntry::Sarc(map) => {
                            let mut sarc = self
                                .open_or_create_sarc(&base_path, self.trim_prefixes(&root))
                                .with_context(|| {
                                    format!(
                                        "Failed to open or create SARC at {}",
                                        base_path.display()
                                    )
                                })?;
                            map.iter().try_for_each(|(nest_root, contents)| {
                                handle_diff_entry(&mut sarc, nest_root, contents).with_context(
                                    || format!("Failed to process {} in {}", nest_root, root),
                                )
                            })?;
                            fs::write(&base_path, compress_if(&sarc.to_binary(), &root))?;
                        }
                        AampDiffEntry::Aamp(plist) => {
                            let pio = ASList::try_from(&ParameterIO::from_binary(
                                self.get_master_bytes(self.trim_prefixes(&root))?,
                            )?)?;
                            let diff = ASList::try_from(&ParameterIO::new().with_root(plist))?;
                            let data = pio
                                .merge(&diff)
                                .into_binary(uk_content::prelude::Endian::Little);
                            let data = compress_if(&data, &root);
                            fs::write(base_path, data)?;
                        }
                    }
                    Ok(())
                })?;
        }
        Ok(())
    }
}
