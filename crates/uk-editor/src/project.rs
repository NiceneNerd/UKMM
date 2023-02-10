use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use fs_err as fs;
use rayon::prelude::*;
use uk_content::{resource::ResourceData, util::IndexMap};
use uk_manager::{core::Manager, settings::Platform};
use uk_mod::{pack::sanitise, unpack::ParallelZipReader, Manifest, Meta};

#[derive(Debug, Clone)]
pub struct Project {
    pub path:     PathBuf,
    pub meta:     Meta,
    pub manifest: Manifest,
}

impl Project {
    pub fn new(name: &str, path: &Path, platform: Platform) -> Self {
        Project {
            path:     path.join(name),
            meta:     Meta {
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
            manifest: Manifest::default(),
        }
    }

    #[allow(irrefutable_let_patterns)]
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
        let manifest: Manifest = serde_yaml::from_str(std::str::from_utf8(
            &zip.get_file("manifest.yml")
                .context("Mod missing manifest")?,
        )?)
        .context("Failed to parse mod manifest")?;
        let path = core.settings().projects_dir().join(sanitise(&meta.name));
        zip.iter().par_bridge().try_for_each(|file| -> Result<()> {
            if matches!(file.file_name().unwrap_or_default().to_str().unwrap_or_default(), "meta.yml" | "manifest.yml") {
                return Ok(());
            }
            let dest = path.join(file);
            let data = zip
                .get_file(file)
                .with_context(|| format!("Failed to read file {} from ZIP", file.display()))
                .and_then(|bytes| {
                    uk_mod::zstd::decode_all(bytes.as_slice()).with_context(|| {
                        format!("Failed to decompress contents of {} in ZIP", file.display())
                    })
                })?;
            let resource: ResourceData = minicbor_ser::from_slice(&data).with_context(|| format!("Failed to parse resource {}", file.display()))?;
            let data = match resource {
                ResourceData::Binary(bin) => bin,
                res => ron::ser::to_string_pretty(&res, Default::default()).expect("Failed to serialize resource to RON").into(),
            };
            if let parent = dest.parent().map(|p| p.to_path_buf()).unwrap_or_default() && !parent.exists() {
                fs::create_dir_all(&parent).with_context(|| format!("Failed to create output directory at {}", parent.display()))?;
            }
            fs::write(dest, data)
                .with_context(|| format!("Failed to extract file {}", file.display()))?;
            Ok(())
        })?;
        Ok(Self {
            path,
            meta,
            manifest,
        })
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
