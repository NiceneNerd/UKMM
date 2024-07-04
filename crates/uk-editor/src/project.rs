use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use uk_content::{resource::ResourceData, util::IndexMap};
use uk_manager::{core::Manager, settings::Platform};
use uk_mod::{pack::sanitise, unpack::ParallelZipReader, Meta};

#[derive(Debug, Clone)]
pub struct Project {
    pub path: PathBuf,
    pub meta: Meta,
    pub files: BTreeSet<PathBuf>,
}

impl Project {
    pub fn new(name: &str, path: &Path, platform: Platform) -> Self {
        Project {
            path: path.join(name),
            meta: Meta {
                api: env!("CARGO_PKG_VERSION").into(),
                name: name.into(),
                author: Default::default(),
                category: Default::default(),
                description: Default::default(),
                masters: IndexMap::default(),
                options: Default::default(),
                platform: uk_mod::ModPlatform::Specific(platform.into()),
                url: Default::default(),
                version: "0.1.0".into(),
            },
            files: Default::default(),
        }
    }

    pub fn open(path: &Path) -> Result<Self> {
        let meta: Meta = serde_yaml::from_str(
            &fs::read_to_string(path.join("meta.yml")).context("Failed to open meta file")?,
        )
        .context("Failed to parse meta file")?;
        let files = jwalk::WalkDir::new(path)
            .into_iter()
            .filter_map(|e| {
                e.ok().and_then(|e| {
                    (e.file_type().is_file()
                        && !e
                            .file_name
                            .to_str()
                            .map(|n| n.ends_with(".yml"))
                            .unwrap_or(true))
                    .then(|| e.path().strip_prefix(path).unwrap().to_path_buf())
                })
            })
            .collect();
        Ok(Self {
            path: path.to_path_buf(),
            meta,
            files,
        })
    }

    pub fn from_mod(core: &Manager, mod_: &Path) -> Result<Self> {
        let zip = ParallelZipReader::open(mod_, false).context("Failed to open ZIP file")?;
        let meta: Meta = serde_yaml::from_str(
            std::str::from_utf8(&zip.get_file("meta.yml").context("Mod missing meta file")?).map(
                |s| {
                    dbg!(s);
                    s
                },
            )?,
        )
        .context("Failed to parse mod meta")?;
        let path = core.settings().projects_dir().join(sanitise(&meta.name));
        let decomp = uk_mod::unpack::init_decompressor();
        let files = zip
            .iter()
            .par_bridge()
            .filter_map(|file| -> Option<Result<PathBuf>> {
                let do_it = || -> Result<Option<PathBuf>> {
                    let dest = path.join(file);
                    dest.parent().map(fs::create_dir_all).transpose()?;
                    let data = zip.get_file(file).with_context(|| {
                        format!("Failed to read file {} from ZIP", file.display())
                    })?;
                    let file_name = file
                        .file_name()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default();
                    if file_name.ends_with(".yml") || file_name.starts_with("thumb.") {
                        fs::write(dest, data).with_context(|| {
                            format!("Failed to extract file {}", file.display())
                        })?;
                        return Ok(None);
                    }
                    let decomp_size =
                        uk_mod::zstd::bulk::Decompressor::upper_bound(data.as_slice())
                            .unwrap_or(data.len() * 1024);
                    let decomp_data = decomp
                        .lock()
                        .decompress(data.as_slice(), decomp_size)
                        .or_else(|_| uk_mod::zstd::decode_all(data.as_slice()))
                        .with_context(|| {
                            format!("Failed to decompress contents of {} in ZIP", file.display())
                        })?;
                    let resource: ResourceData = minicbor_ser::from_slice(&decomp_data)
                        .with_context(|| format!("Failed to parse resource {}", file.display()))?;
                    let data = match resource {
                        ResourceData::Binary(bin) => bin,
                        res => ron::ser::to_string_pretty(&res, Default::default())
                            .expect("Failed to serialize resource to RON")
                            .into(),
                    };
                    fs::write(dest, data)
                        .with_context(|| format!("Failed to extract file {}", file.display()))?;
                    Ok(Some(file.to_path_buf()))
                };
                do_it().transpose()
            })
            .collect::<Result<_>>()?;
        Ok(Self { path, meta, files })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn project_from_mod() {
        let mod_ = "/home/nn/.local/share/ukmm/wiiu/mods/tmp3eSZKv";
        let core = uk_manager::core::Manager::init().unwrap();
        let project = super::Project::from_mod(&core, mod_.as_ref()).unwrap();
        dbg!(project);
    }
}
