use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use roead::{
    aamp::{ParameterIO, ParameterList, ParameterListing},
    sarc::{Sarc, SarcWriter},
    yaz0::compress_if,
};
use rustc_hash::FxHashMap;
use tempfile::tempdir;
use uk_reader::ResourceReader;

use crate::{settings::Platform, util::extract_7z};
mod actorinfo;
mod areadata;
mod deepmerge;
mod drops;
mod dstatic;
mod events;
mod mainstatic;
mod maps;
mod packs;
mod quests;
mod residents;
mod shops;
mod texts;

type AampDiffMap = FxHashMap<String, AampDiffEntry>;

pub enum AampDiffEntry {
    Sarc(AampDiffMap),
    Aamp(ParameterList),
}

impl AampDiffEntry {
    pub fn as_mut_sarc(&mut self) -> &mut AampDiffMap {
        if let AampDiffEntry::Sarc(ref mut map) = self {
            map
        } else {
            panic!("Not a SARC entry")
        }
    }
}

pub fn parse_aamp_diff(header_name: &str, pio: &ParameterIO) -> Result<AampDiffMap> {
    pio.object(header_name)
        .context("Deepmerge log missing file table")?
        .0
        .values()
        .filter_map(|s| s.as_str().ok())
        .try_fold(
            FxHashMap::default(),
            |mut acc, file| -> Result<FxHashMap<String, AampDiffEntry>> {
                let parts = file.split("//").collect::<Vec<_>>();
                if parts.is_empty() {
                    anyhow::bail!("Why are there no diff path parts?");
                }
                let root_path = parts[0];
                let root = acc
                    .entry(root_path.to_string())
                    .or_insert_with(|| AampDiffEntry::Sarc(Default::default()))
                    .as_mut_sarc();
                let parent = if parts.len() == 3 {
                    root.entry(parts[1].into())
                        .or_insert_with(|| AampDiffEntry::Sarc(Default::default()))
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
                parent.insert(
                    parts[parts.len() - 1].to_string(),
                    AampDiffEntry::Aamp(plist),
                );
                Ok(acc)
            },
        )
}

#[derive(Debug)]
struct BnpConverter {
    dump: Arc<ResourceReader>,
    platform: Platform,
    path: PathBuf,
    content: &'static str,
    aoc: &'static str,
}

impl BnpConverter {
    #[inline(always)]
    fn trim_prefixes<'f>(&self, file: &'f str) -> &'f str {
        file.trim_start_matches(self.content)
            .trim_start_matches(self.aoc)
            .trim_start_matches('/')
            .trim_start_matches('\\')
    }

    fn open_or_create_sarc(&self, dest_path: &Path, root_path: &str) -> Result<SarcWriter> {
        let base_sarc = self.dump.get_bytes_uncached(root_path);
        if !dest_path.exists() {
            let base_sarc = base_sarc?;
            fs::write(dest_path, &base_sarc)?;
            Ok(SarcWriter::from_sarc(&Sarc::new(&base_sarc)?))
        } else {
            let stripped = Sarc::new(fs::read(dest_path)?)?;
            let mut sarc = SarcWriter::from_sarc(&stripped);
            if let Ok(base_sarc) =
                base_sarc.and_then(|data| Ok(Sarc::new(data).map_err(anyhow::Error::from)?))
            {
                sarc.files.extend(base_sarc.files().filter_map(|file| {
                    file.name().and_then(|name| {
                        match (stripped.get(name), file.is_sarc()) {
                            // If it's not in the stripped SARC, add it from the base game
                            (None, _) => Some((name.into(), file.data.to_vec())),
                            // If it is in the stripped SARC, but it's not a nested SARC, skip it
                            (Some(_), false) => None,
                            // If it is in the stripped SARC, and it's a nested SARC, fill it up
                            (Some(stripped_file), true) => {
                                let stripped_nested = Sarc::new(stripped_file.data).ok()?;
                                let base_nested = Sarc::new(file.data).ok()?;
                                let mut nested_merged = SarcWriter::from_sarc(&base_nested);
                                nested_merged
                                    .files
                                    .extend(stripped_nested.files().filter_map(|file| {
                                        file.name().map(|name| (name.into(), file.data.to_vec()))
                                    }));
                                Some((
                                    name.into(),
                                    compress_if(&nested_merged.to_binary(), name).to_vec(),
                                ))
                            }
                        }
                    })
                }))
            }
            Ok(sarc)
        }
    }

    fn inject_into_sarc(&self, nest_path: &str, data: Vec<u8>, dlc: bool) -> Result<()> {
        let parts = nest_path.split("//").collect::<Vec<_>>();
        if parts.len() < 2 {
            anyhow::bail!("Bad nested path: {}", nest_path);
        }
        let base_path = self
            .path
            .join(if dlc { self.aoc } else { self.content })
            .join(parts[0]);
        let mut sarc = self.open_or_create_sarc(&base_path, parts[0])?;
        let mut nested = None;
        if parts.len() == 3 {
            let nested_path = parts[1];
            nested = Some(SarcWriter::from_sarc(&Sarc::new(
                sarc.get_file(nested_path).context("Missing nested SARC")?,
            )?));
        }
        let parent = nested.as_mut().unwrap_or(&mut sarc);
        let dest_path = *parts.iter().last().expect("This exists");
        let data = compress_if(&data, dest_path);
        parent.files.insert(dest_path.into(), data.to_vec());
        if let Some(mut nested) = nested {
            let nested_path = parts[1];
            sarc.add_file(nested_path, compress_if(&nested.to_binary(), nested_path));
        }
        fs::write(&base_path, compress_if(&sarc.to_binary(), &base_path))?;
        Ok(())
    }

    fn convert(self) -> Result<PathBuf> {
        println!("Actor info");
        self.handle_actorinfo()?;
        println!("Areadata");
        self.handle_areadata()?;
        println!("Deepmerge");
        self.handle_deepmerge()?;
        println!("Drops");
        self.handle_drops()?;
        println!("DungeonStatic");
        self.handle_dungeon_static()?;
        println!("EventInfo");
        self.handle_events()?;
        println!("MainfieldStatic");
        self.handle_mainfield_static()?;
        println!("Maps");
        self.handle_maps()?;
        println!("Quests");
        self.handle_quests()?;
        println!("Residents");
        self.handle_residents()?;
        println!("Shops");
        self.handle_shops()?;
        Ok(todo!())
    }
}

pub fn convert_bnp(core: &crate::core::Manager, path: &Path) -> Result<PathBuf> {
    let tempdir = tempdir()?.into_path();
    dbg!(&tempdir);
    extract_7z(path, &tempdir).context("Failed to extract BNP")?;
    let (content, aoc) = uk_content::platform_prefixes(core.settings().current_mode.into());
    let converter = BnpConverter {
        platform: core.settings().current_mode,
        dump: core.settings().dump().context("No dump for current mode")?,
        path: tempdir,
        content,
        aoc,
    };
    converter.convert()
}

#[cfg(test)]
#[test]
fn test_convert() {
    let path = dirs2::download_dir().unwrap().join("SecondWindv1.9.13.bnp");
    convert_bnp(&super::core::Manager::init().unwrap(), path.as_ref()).unwrap();
}
