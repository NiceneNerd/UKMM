use anyhow_ext::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::{
    aamp::{get_default_name_table, ParameterIO, ParameterList, ParameterListing},
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use uk_content::{
    actor::params::shop::*,
    prelude::{Resource, String64},
    util::merge_plist,
};
use uk_util::OptionExt;

use super::{parse_aamp_diff, AampDiffEntry, BnpConverter};

fn update_name_table(list: &ParameterList, base: Option<&ShopData>) {
    let name_table = get_default_name_table();
    for obj in list.objects.0.values() {
        for param in obj.0.values() {
            if let Ok(string) = param.as_str() {
                name_table.add_name(string.to_string());
            }
        }
    }
    for list in list.lists.0.values() {
        update_name_table(list, None);
    }
    if let Some(base) = base {
        for table in base.0.values().flatten() {
            for name in table.keys() {
                name_table.add_name(name.as_str().to_string());
            }
        }
    }
}

fn merge(data: &mut ShopData, diff: &ParameterList) -> Result<()> {
    update_name_table(diff, Some(data));
    for (name, table) in diff
        .list("Additions")
        .context("Shop diff missing Additions")?
        .lists
        .iter_by_name()
    {
        let name: String64 = name.expect("Bad shop diff").as_ref().into();
        #[allow(unstable_name_collisions)]
        let base = data.0.entry(name).or_default().get_or_insert_default();
        for (i, (name, params)) in table.objects.iter_by_name().enumerate() {
            let name: String64 = name
                .ok()
                .map(|n| n.as_ref().into())
                .or_else(|| {
                    params
                        .get("ItemName")
                        .and_then(|n| n.as_str().ok())
                        .map(|n| n.into())
                })
                .expect("Bad shop diff");
            let num = params.get("ItemNum");
            let adjust = params.get("ItemAdjustPrice");
            let look_get = params.get("ItemLookGetFlg");
            let amount = params.get("ItemAmount");
            match base.get_mut(&name) {
                Some(base) => {
                    base.sort = i as i32;
                    if let Some(num) = num {
                        base.num = num.as_int()?;
                    }
                    if let Some(adjust) = adjust {
                        base.adjust_price = adjust.as_int()?;
                    }
                    if let Some(look_get) = look_get {
                        base.look_get_flag = look_get.as_bool()?;
                    }
                    if let Some(amount) = amount {
                        base.amount = amount.as_int()?;
                    }
                }
                None => {
                    base.insert(name, ShopItem {
                        sort: i as i32,
                        num: num.context("Shop diff item missing num")?.as_int()?,
                        adjust_price: adjust
                            .context("Shop diff item missing adjust_price")?
                            .as_int()?,
                        look_get_flag: look_get
                            .context("Shop diff item missing look_get_flag")?
                            .as_bool()?,
                        amount: amount.context("Shop diff item missing amount")?.as_int()?,
                        delete: false,
                    });
                }
            }
        }
    }
    for (name, table) in diff
        .list("Removals")
        .context("Shop diff missing Removals")?
        .lists
        .iter_by_name()
    {
        let name: String64 = name.expect("Bad shop diff").as_ref().into();
        if let Some(base) = data.0.get_mut(&name).and_then(|n| n.as_mut()) {
            for name in table.objects.iter_by_name().filter_map(|(k, _)| k.ok()) {
                let name: String64 = name.as_ref().into();
                if let Some(base) = base.get_mut(&name) {
                    base.delete = true;
                }
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
                handle_diff_entry(&mut nest_sarc, nested_file, nested_contents)
                    .with_context(|| format!("Failed to process {}", nested_file))?;
            }
            let data = nest_sarc.to_binary();
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
        AampDiffEntry::Aamp(plist) => {
            let mut base = ShopData::from_binary(nested_bytes)?;
            merge(&mut base, plist)?;
            let data = base.into_binary(uk_content::prelude::Endian::Little);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
    }
    Ok(())
}

impl BnpConverter {
    pub fn handle_shops(&self) -> Result<()> {
        let shops_path = self.current_root.join("logs/shop.aamp");
        if shops_path.exists() {
            log::debug!("Processing shops log");
            let pio = ParameterIO::from_binary(fs::read(shops_path)?)?;
            let diff = parse_aamp_diff("Filenames", &pio)?;
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
                            let mut pio = ParameterIO::from_binary(
                                self.get_master_bytes(self.trim_prefixes(&root))?,
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
