use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::{
    aamp::{Parameter, ParameterIO, ParameterList},
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use rustc_hash::FxHashSet;
use uk_content::{
    prelude::{Mergeable, Resource, String64},
    resource::ASList,
    util::{IndexMap, IndexSet, ParameterExt},
};

use super::{parse_aamp_diff, AampDiffEntry, BnpConverter};

fn merge_plist(base: &mut ParameterList, diff: &ParameterList) -> Result<()> {
    fn merge_addres(base: &mut ParameterList, diff: &ParameterList) {
        let bfres: IndexSet<String64> = base
            .objects
            .0
            .values()
            .chain(diff.objects.0.values())
            .filter_map(|obj| obj.get("Anim").and_then(|a| a.as_safe_string().ok()))
            .collect();
        for (i, v) in bfres.into_iter().enumerate() {
            let key = format!("AddRes_{}", i);
            let obj = base.objects.entry(key).or_default();
            obj.insert("Anim", Parameter::String64(v.into()));
        }
    }

    fn merge_asdefine(base: &mut ParameterList, diff: &ParameterList) {
        let listing: IndexMap<String64, usize> = base
            .objects
            .0
            .values()
            .enumerate()
            .filter_map(|(i, obj)| {
                obj.get("Name")
                    .and_then(|n| n.as_safe_string().ok().map(|n| (n, i)))
            })
            .collect();
        let defs: IndexMap<String64, String64> = diff
            .objects
            .0
            .values()
            .filter_map(|obj| {
                obj.get("Name")
                    .and_then(|n| n.as_safe_string().ok())
                    .and_then(|n| {
                        obj.get("Filename")
                            .and_then(|f| f.as_safe_string().ok())
                            .map(|f| (n, f))
                    })
            })
            .collect();
        let mut new_idx = listing.len();
        for (k, v) in defs {
            let key;
            if let Some(index) = listing.get(&k) {
                key = format!("ASDefine_{}", index);
            } else {
                key = format!("ASDefine_{}", new_idx);
                let obj = base.objects.entry(key.clone()).or_default();
                obj.insert("Name", k.into());
                new_idx += 1;
            }
            if let Some(obj) = base.objects.get_mut(&key) {
                obj.insert("Filename", v.into());
            }
        }
    }

    Ok(())
}

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
    #[allow(irrefutable_let_patterns)]
    pub fn handle_aslist(&self) -> Result<()> {
        let aslist_path = self.path.join("logs/aslist.aamp");
        if aslist_path.exists() {
            let pio = ParameterIO::from_binary(fs::read(aslist_path)?)?;
            let diff = parse_aamp_diff("FileTable", &pio)?;
            diff.into_par_iter()
                .try_for_each(|(root, contents)| -> Result<()> {
                    let base_path = self.path.join(&root);
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
                            let pio = ASList::try_from(&ParameterIO::from_binary(
                                self.dump.get_bytes_uncached(self.trim_prefixes(&root))?,
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
