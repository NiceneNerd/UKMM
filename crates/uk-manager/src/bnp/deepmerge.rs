use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::{
    aamp::ParameterIO,
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use uk_content::util::merge_plist;

use super::{parse_aamp_diff, AampDiffEntry, BnpConverter};

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
                handle_diff_entry(&mut nest_sarc, nested_file, nested_contents)?;
            }
            let data = nest_sarc.to_binary();
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
        AampDiffEntry::Aamp(plist) => {
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
        let deepmerge_path = self.current_root.join("logs/deepmerge.aamp");
        if deepmerge_path.exists() {
            log::debug!("Processing deepmerge log");
            let pio = ParameterIO::from_binary(fs::read(deepmerge_path)?)?;
            let diff = parse_aamp_diff("FileTable", &pio)?;
            diff.into_par_iter()
                .try_for_each(|(root, contents)| -> Result<()> {
                    let base_path = self.current_root.join(&root);
                    base_path.parent().iter().try_for_each(fs::create_dir_all)?;
                    match contents {
                        AampDiffEntry::Sarc(map) => {
                            let mut sarc =
                                self.open_or_create_sarc(&base_path, self.trim_prefixes(&root))?;
                            map.iter().try_for_each(|(nest_root, contents)| {
                                handle_diff_entry(&mut sarc, nest_root, contents)
                            })?;
                            fs::write(&base_path, compress_if(&sarc.to_binary(), &root))?;
                        }
                        AampDiffEntry::Aamp(plist) => {
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
