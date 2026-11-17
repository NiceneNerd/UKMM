//! End-to-end scenario against a real game dump. Runs only when
//! `UKMM_TEST_DUMP` points at one: a directory containing the unpacked base
//! game (`content/…`, optionally `aoc/0010/…`), or a `.wua` archive.
//!
//! Drives the full cycle with no UI: temp config → configure dump → install a
//! mod → enable → reorder → apply (remerge) → deploy, and asserts the merged
//! output.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use uk_manager::{
    core::Manager,
    mods::LookupMod,
    settings::{DeployConfig, DeployLayout, DeployMethod, PlatformSettings},
};
use uk_mod::{Manifest, Meta, ModPlatform};
use uk_reader::ResourceReader;

fn dump_reader(dump: &Path) -> ResourceReader {
    if dump.is_file() {
        ResourceReader::from_zarchive(dump).unwrap()
    } else {
        ResourceReader::from_unpacked_dirs(
            Some(dump.join("content")),
            dump.join("aoc/0010")
                .exists()
                .then(|| dump.join("aoc/0010")),
            None::<PathBuf>,
            uk_content::prelude::Endian::Big,
        )
        .unwrap()
    }
}

fn mod_zip(dir: &Path, name: &str) -> PathBuf {
    let mut meta = Meta::blank(ModPlatform::Universal);
    meta.name = "E2E Test Mod".into();
    let path = dir.join(name);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("meta.yml", opts).unwrap();
    zip.write_all(serde_yaml::to_string(&meta).unwrap().as_bytes())
        .unwrap();
    zip.start_file("manifest.yml", opts).unwrap();
    zip.write_all(
        serde_yaml::to_string(&Manifest::default())
            .unwrap()
            .as_bytes(),
    )
    .unwrap();
    zip.finish().unwrap();
    path
}

#[test]
fn install_reorder_apply_deploy() {
    let Some(dump) = std::env::var_os("UKMM_TEST_DUMP").map(PathBuf::from) else {
        eprintln!("UKMM_TEST_DUMP not set, skipping e2e test");
        return;
    };
    assert!(
        dump.exists(),
        "UKMM_TEST_DUMP={} does not exist",
        dump.display()
    );

    let dir = tempfile::tempdir().unwrap();
    let out = dir.path().join("deploy");
    let settings =
        uk_manager::settings::Settings::load_from(&dir.path().join("config").join("settings.yml"));
    {
        let mut s = settings.write();
        s.wiiu_config = Some(PlatformSettings {
            language: uk_content::constants::Language::USen,
            profile: "Default".into(),
            dump: std::sync::Arc::new(dump_reader(&dump)),
            deploy_config: Some(DeployConfig {
                output: out.clone(),
                method: DeployMethod::Copy,
                auto: false,
                cemu_rules: false,
                executable: None,
                layout: DeployLayout::WithoutName,
            }),
        });
        s.save().unwrap();
    }
    let core = Manager::init_with(settings).unwrap();

    // install two mods
    let mod_a = mod_zip(dir.path(), "a.zip");
    let mod_b = mod_zip(dir.path(), "b.zip");
    {
        let mods = core.mod_manager();
        let a = mods.add(&mod_a, None).unwrap();
        let b = mods.add(&mod_b, None).unwrap();
        mods.set_enabled(a.as_map_id(), true, None).unwrap();
        mods.set_enabled(b.as_map_id(), true, None).unwrap();
        mods.save().unwrap();
        // reorder: b before a
        let mut order = core.mod_manager().all_mods().collect::<Vec<_>>();
        uk_app::move_selected(&mut order, std::slice::from_ref(&b), 0);
        let hashes = order.iter().map(|m| m.hash()).collect();
        core.mod_manager().set_order(hashes);
        core.mod_manager().save().unwrap();
    }

    // remerge + deploy through the app layer
    uk_app::apply_changes(&core, vec![], None).unwrap();
    core.deploy_manager().deploy().unwrap();

    // the merged output exists and the profile persisted
    let profile = core.settings().profile_dir().join("profile.yml");
    assert!(profile.exists());
    let merged_content = core.settings().merged_dir().join("content");
    assert!(
        merged_content.exists(),
        "merged output missing at {}",
        merged_content.display()
    );
}
