use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use fs_err as fs;
use parking_lot::Mutex;
use rayon::prelude::*;
use roead::{
    aamp::{ParameterIO, ParameterList, ParameterListing},
    sarc::{File, Sarc, SarcWriter},
    yaz0::compress_if,
};
use rustc_hash::{FxHashMap, FxHashSet};
use uk_mod::pack::ModPacker;
use uk_reader::ResourceReader;

use crate::{
    settings::{Language, Platform},
    util::{extract_7z, get_temp_file},
};
mod actorinfo;
mod areadata;
mod aslist;
mod deepmerge;
mod drops;
mod dstatic;
mod effects;
mod events;
mod gamedata;
mod mainstatic;
mod maps;
mod quests;
mod residents;
mod savedata;
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
    game_lang: Language,
    platform: Platform,
    path: PathBuf,
    content: &'static str,
    aoc: &'static str,
    packs: Arc<Mutex<FxHashSet<PathBuf>>>,
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
        #[inline(always)]
        fn is_stripped_sarc(file: &File) -> bool {
            static BCML_SARC_EXTS: &[&str] = &[
                "sarc",
                "pack",
                "bactorpack",
                "bmodelsh",
                "stats",
                "ssarc",
                "sbactorpack",
                "sbmodelsh",
                "sstats",
                "sblarc",
                "blarc",
            ];
            static BCML_SPECIAL: &[&str] = &[
                "gamedata",
                "savedataformat",
                "tera_resource.Nin_NX_NVN",
                "Dungeon",
                "Bootup_",
                "AocMainField",
            ];
            file.is_sarc()
                && file
                    .name
                    .map(|n| {
                        let name = Path::new(n);
                        name.extension()
                            .and_then(|e| e.to_str())
                            .map(|e| BCML_SARC_EXTS.contains(&e))
                            .unwrap_or(false)
                            && !BCML_SPECIAL.iter().any(|xn| n.starts_with(xn))
                    })
                    .unwrap_or(false)
        }

        let base_sarc = self.dump.get_bytes_uncached(root_path);
        if !dest_path.exists() {
            let base_sarc = base_sarc?;
            fs::write(dest_path, &base_sarc)?;
            Ok(SarcWriter::from_sarc(&Sarc::new(&base_sarc)?))
        } else {
            self.packs.lock().remove(dest_path);
            let stripped = Sarc::new(fs::read(dest_path)?)?;
            let mut sarc = SarcWriter::from_sarc(&stripped);
            if let Ok(base_sarc) =
                base_sarc.and_then(|data| Ok(Sarc::new(data).map_err(anyhow::Error::from)?))
            {
                sarc.files.extend(base_sarc.files().filter_map(|file| {
                    file.name().and_then(|name| {
                        match (stripped.get(name), is_stripped_sarc(&file)) {
                            // If it's not in the stripped SARC, add it from the base game
                            (None, _) => Some((name.into(), file.data.to_vec())),
                            // If it is in the stripped SARC, but it's not a nested SARC, skip it
                            (Some(_), false) => None,
                            // If it is in the stripped SARC, and it's a nested SARC, fill it up
                            (Some(stripped_file), true) => {
                                let stripped_nested = Sarc::new(stripped_file.data).ok()?;
                                let base_nested = Sarc::new(file.data).ok()?;
                                let mut nested_merged = SarcWriter::from_sarc(&base_nested);
                                if name.ends_with("arc") {
                                    nested_merged.set_legacy_mode(true);
                                }
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

    fn convert_root(&self) -> Result<()> {
        let packs_path = self.path.join("logs/packs.json");
        if packs_path.exists() {
            let log: FxHashMap<String, String> = serde_json::from_str(
                &fs::read_to_string(packs_path).context("Failed to read packs.json")?,
            )
            .context("Failed to parse packs.json")?;
            self.packs.lock().extend(
                log.into_values()
                    .map(|p| self.path.join(p.replace('\\', "/"))),
            );
        };

        self.handle_actorinfo()
            .context("Failed to process actor info log")?;
        self.handle_aslist()
            .context("Failed to process AS list log")?;
        self.handle_areadata()
            .context("Failed to process areadata log")?;
        self.handle_deepmerge()
            .context("Failed to process deepmerge log")?;
        self.handle_drops().context("Failed to process drops log")?;
        self.handle_dungeon_static()
            .context("Failed to process dungeon static log")?;
        self.handle_events()
            .context("Failed to process eventinfo log")?;
        self.handle_gamedata()
            .context("Failed to process gamedata log")?;
        self.handle_mainfield_static()
            .context("Failed to process mainfield static log")?;
        self.handle_maps().context("Failed to process maps log")?;
        self.handle_quests()
            .context("Failed to process quests log")?;
        self.handle_residents()
            .context("Failed to process residents log")?;
        self.handle_savedata()
            .context("Failed to process savedata log")?;
        self.handle_shops().context("Failed to process shops log")?;
        self.handle_effects()
            .context("Failed to process status effect log")?;
        self.handle_texts().context("Failed to process texts log")?;

        let packs = self.packs.lock().clone();
        self.packs.lock().clear();

        packs.into_par_iter().try_for_each(|file| -> Result<()> {
            let mut sarc = self.open_or_create_sarc(
                &file,
                self.trim_prefixes(
                    file.strip_prefix(&self.path)
                        .expect("Impossible")
                        .to_str()
                        .unwrap_or_default(),
                ),
            )?;
            let data = sarc.to_binary();
            let data = compress_if(&data, &file);
            fs::write(file, data)?;
            Ok(())
        })?;
        Ok(())
    }

    fn convert(mut self) -> Result<PathBuf> {
        let root = self.path.clone();
        self.convert_root()?;

        let opt_dir = root.join("options");
        if opt_dir.exists() {
            for option in fs::read_dir(opt_dir)?.filter_map(|r| {
                r.ok().and_then(|r| {
                    let path = r.path();
                    path.is_dir().then_some(path)
                })
            }) {
                log::info!(
                    "Processing BNP logs for option {}",
                    option
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or_default()
                );
                self.path = option;
                self.convert_root()?;
            }
        }
        Ok(root)
    }
}

pub fn unpack_bnp(core: &crate::core::Manager, path: &Path) -> Result<PathBuf> {
    let tempdir = crate::util::get_temp_folder();
    log::info!("Extracting BNP…");
    extract_7z(path, &tempdir).context("Failed to extract BNP")?;
    let (content, aoc) = uk_content::platform_prefixes(core.settings().current_mode.into());
    log::info!("Processing BNP logs…");
    let converter = BnpConverter {
        platform: core.settings().current_mode,
        game_lang: core
            .settings()
            .platform_config()
            .context("No config for current platform")?
            .language,
        dump: core.settings().dump().context("No dump for current mode")?,
        content,
        aoc,
        packs: Default::default(),
        path: tempdir.clone(),
    };
    let path = converter.convert()?;
    log::info!("BNP unpacked");
    Ok(path)
}

pub fn convert_bnp(core: &crate::core::Manager, path: &Path) -> Result<PathBuf> {
    let tempdir = unpack_bnp(core, path).context("Failed to unpack BNP")?;
    let tempfile = get_temp_file();
    let meta =
        ModPacker::parse_info(tempdir.join("info.json")).context("Failed to parse BNP metadata")?;
    let new_mod = ModPacker::new(tempdir, tempfile.as_path(), Some(meta), vec![
        core.settings()
            .dump()
            .context("No dump for current platform")?,
    ])
    .context("Failed to package converted BNP")?;
    new_mod.pack()
}

#[cfg(test)]
#[test]
fn test_convert() {
    let path = dirs2::download_dir()
        .unwrap()
        .join("clearcameraui_nodetection.bnp"); // join("rebalance.bnp"); //("SecondWindv1.9.13.bnp");
    unpack_bnp(&super::core::Manager::init().unwrap(), path.as_ref()).unwrap();
}
