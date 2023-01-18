use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use roead::{
    aamp::{ParameterIO, ParameterList, ParameterListing, ParameterObject},
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use rustc_hash::FxHashMap;
use uk_content::{
    actor::params::shop::*,
    prelude::{Mergeable, Resource, String64},
    util::{merge_plist, IndexMap},
};

use super::{parse_aamp_diff, AampDiffEntry, BnpConverter};

fn plist_to_diff(list: &ParameterList) -> Result<ShopData> {
    fn pobj_to_item(index: usize, obj: &ParameterObject, removals: bool) -> Result<ShopItem> {
        Ok(ShopItem {
            sort: index as i32,
            num: obj
                .get("ItemNum")
                .context("Shop item missing ItemNum")?
                .as_int()?,
            adjust_price: obj
                .get("ItemAdjustPrice")
                .context("Shop item missing ItemAdjustPrice")?
                .as_int()?,
            look_get_flag: obj
                .get("ItemLookGetFlg")
                .context("Shop item missing ItemLookGetFlg")?
                .as_bool()?,
            amount: obj
                .get("ItemAmount")
                .context("Shop item missing ItemAmount")?
                .as_int()?,
            delete: removals,
        })
    }

    let mut shop_data: IndexMap<String64, Option<ShopTable>> = list
        .list("Additions")
        .context("Shop diff missing Additions")?
        .lists
        .iter_by_name()
        .map(|(n, list)| -> Result<(String64, Option<ShopTable>)> {
            Ok((
                n.map(|n| n.as_ref().into()).unwrap_or_default(),
                Some(
                    list.objects
                        .0
                        .values()
                        .enumerate()
                        .map(|(i, obj)| -> Result<(String64, ShopItem)> {
                            let name = obj
                                .get("ItemName")
                                .context("Shop item missing name")?
                                .as_string64()?;
                            Ok((*name, pobj_to_item(i, obj, false)?))
                        })
                        .collect::<Result<_>>()?,
                ),
            ))
        })
        .collect::<Result<_>>()?;
    for (name, list) in list
        .list("Removals")
        .context("Shop diff missing Removals")?
        .lists
        .iter_by_name()
    {
        let name = name.map(|n| n.as_ref().into()).unwrap_or_default();
        let table = shop_data.entry(name).or_default().get_or_insert_default();
        table.extend(
            list.objects
                .0
                .values()
                .enumerate()
                .map(|(i, obj)| -> Result<(String64, ShopItem)> {
                    let name = obj
                        .get("ItemName")
                        .context("Shop item missing name")?
                        .as_string64()?;
                    Ok((*name, pobj_to_item(i, obj, true)?))
                })
                .collect::<Result<IndexMap<String64, ShopItem>>>()?,
        );
    }
    Ok(ShopData(shop_data))
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
            let base = ShopData::from_binary(nested_bytes)?;
            let diff = plist_to_diff(plist)?;
            let data = base
                .merge(&diff)
                .into_binary(uk_content::prelude::Endian::Little);
            let data = compress_if(&data, nest_root);
            sarc.files.insert(nest_root.into(), data.to_vec());
        }
    }
    Ok(())
}

impl BnpConverter {
    pub fn handle_shops(&self) -> Result<()> {
        let shops_path = self.path.join("logs/shop.aamp");
        if shops_path.exists() {
            let pio = ParameterIO::from_binary(fs::read(shops_path)?)?;
            let diff = parse_aamp_diff("Filenames", &pio)?;
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
