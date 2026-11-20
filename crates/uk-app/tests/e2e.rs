//! End-to-end scenario against a real game dump. Runs only when
//! `UKMM_TEST_DUMP` points at one: a directory containing the unpacked base
//! game (`content/…`, optionally `aoc/0010/…`), or a `.wua` archive.
//!
//! Drives the full cycle with no UI: temp config → configure dump → install a
//! mod → enable → reorder → apply (remerge) → deploy, and asserts the merged
//! output.

use std::path::{Path, PathBuf};

use uk_manager::{
    core::Manager,
    mods::LookupMod,
    settings::{DeployConfig, DeployLayout, DeployMethod, PlatformSettings},
};
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

use uk_app::testkit::mod_zip_named;

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
    let mod_a = mod_zip_named(dir.path(), "a.zip", "E2E Test Mod A");
    let mod_b = mod_zip_named(dir.path(), "b.zip", "E2E Test Mod B");
    core.with_mod_manager(|mods| {
        let a = mods.add(&mod_a, None).unwrap();
        let b = mods.add(&mod_b, None).unwrap();
        mods.set_enabled(a.as_map_id(), true, None).unwrap();
        mods.set_enabled(b.as_map_id(), true, None).unwrap();
        mods.save().unwrap();
        // reorder: b before a
        let mut order = mods.all_mods().collect::<Vec<_>>();
        uk_app::move_selected(&mut order, std::slice::from_ref(&b), 0);
        let hashes = order.iter().map(|m| m.hash()).collect();
        mods.set_order(hashes);
        mods.save().unwrap();
    });

    // remerge + deploy through the app layer
    uk_app::apply_changes(&core, vec![], None).unwrap();
    core.with_deploy_manager(|d| d.deploy()).unwrap();

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

/// The same cycle, driven through the session: install → enable → reorder →
/// apply → deploy, with no UI at all.
#[test]
fn session_install_reorder_apply_deploy() {
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
    let core = std::sync::Arc::new(Manager::init_with(settings).unwrap());
    let session = uk_app::Session::open(core.clone()).unwrap();
    let rx = session.subscribe();

    let mod_a = mod_zip_named(dir.path(), "a.zip", "E2E Test Mod A");
    let mod_b = mod_zip_named(dir.path(), "b.zip", "E2E Test Mod B");

    // Install both mods through the session.
    let mut ids = vec![];
    for path in [mod_a, mod_b] {
        let events = run_job(&session, &rx, uk_app::Command::Open { path, meta: None });
        let candidate = events
            .iter()
            .find_map(|event| {
                match event {
                    uk_app::Event::Inspection(uk_app::Inspection::Ready(candidate)) => {
                        Some(candidate.clone())
                    }
                    _ => None,
                }
            })
            .expect("the mod is ready to install");
        let events = run_job(&session, &rx, uk_app::Command::Install(candidate));
        let id = events
            .iter()
            .find_map(|event| {
                match event {
                    uk_app::Event::Installed { id } => Some(*id),
                    _ => None,
                }
            })
            .expect("the mod was installed");
        ids.push(id);
    }
    assert_eq!(session.snapshot().entries.len(), 2);

    // Disable the first mod — the change is saved right away.
    run_job(&session, &rx, uk_app::Command::SetEnabled {
        ids:     vec![ids[0]],
        enabled: false,
    });
    assert!(!session.snapshot().entries[0].enabled);
    run_job(&session, &rx, uk_app::Command::SetEnabled {
        ids:     vec![ids[0]],
        enabled: true,
    });
    assert!(session.snapshot().entries[0].enabled);

    // Reorder: the second mod comes first.
    run_job(&session, &rx, uk_app::Command::MoveTo {
        ids:   vec![ids[1]],
        index: 0,
    });
    let entries = &session.snapshot().entries;
    assert_eq!(entries[0].id, ids[1]);
    assert_eq!(entries[1].id, ids[0]);

    // The profile on disk matches the session's state.
    let profile_path = core.settings().profile_dir().join("profile.yml");
    let profile: uk_manager::mods::Profile =
        serde_yaml::from_str(&std::fs::read_to_string(&profile_path).unwrap()).unwrap();
    assert_eq!(
        profile.load_order().clone(),
        vec![ids[1].0, ids[0].0],
        "profile.yml: {}",
        profile_path.display()
    );

    // Merge and deploy.
    run_job(&session, &rx, uk_app::Command::Apply);
    run_job(&session, &rx, uk_app::Command::Remerge);
    run_job(&session, &rx, uk_app::Command::Deploy);

    let merged_content = core.settings().merged_dir().join("content");
    assert!(
        merged_content.exists(),
        "merged output missing at {}",
        merged_content.display()
    );
    assert!(out.exists(), "deploy output missing at {}", out.display());
}

/// Dispatches a command and collects the events up to its `JobDone`.
fn run_job(
    session: &uk_app::Session,
    rx: &flume::Receiver<uk_app::Event>,
    cmd: uk_app::Command,
) -> Vec<uk_app::Event> {
    let job = session.dispatch(cmd);
    let mut events = vec![];
    loop {
        let event = rx
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("session event");
        let done = matches!(&event, uk_app::Event::JobDone { job: j, .. } if *j == job);
        events.push(event);
        if done {
            break;
        }
    }
    assert!(
        events
            .iter()
            .any(|event| matches!(event, uk_app::Event::JobDone { result: Ok(()), .. })),
        "the job failed: {events:?}"
    );
    events
}
