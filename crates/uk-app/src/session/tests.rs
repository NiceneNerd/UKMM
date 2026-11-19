//! Headless session tests: temp config, no GUI, no game dump (except where a
//! test explicitly builds a synthetic one).

use std::{
    io::Write,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use uk_content::prelude::Endian;
use uk_manager::{core::Manager, settings::Settings};
use uk_mod::{
    ExclusiveOptionGroup, Manifest, Meta, ModOption, ModOptionGroup, ModPlatform, OptionGroup,
};

use crate::{
    error::AppError,
    order::move_ids,
    profile::ProfileName,
    session::{Command, Event, Inspection, ModEntry, ModId, Session, Snapshot},
    view::{Sort, filter_entries, load_index_of, sort_entries},
};

// ---------------------------------------------------------------- fixtures

/// A mod fixture: metadata plus the files it contains.
struct Fixture {
    meta:    Meta,
    files:   Vec<String>,
    options: Vec<ModOption>,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let mut meta = Meta::blank(ModPlatform::Universal);
        meta.name = name.into();
        Self {
            meta,
            files: vec![],
            options: vec![],
        }
    }

    fn with_file(mut self, file: &str) -> Self {
        self.files.push(file.into());
        self
    }

    fn with_platform(mut self, platform: ModPlatform) -> Self {
        self.meta.platform = platform;
        self
    }

    fn with_version(mut self, version: &str) -> Self {
        self.meta.version = version.into();
        self
    }

    fn with_masters(mut self, hash: usize, name: &str, version: &str) -> Self {
        self.meta
            .masters
            .insert(hash, (name.into(), version.into()));
        self
    }

    /// Adds a required option group with a single option.
    fn with_required_option(mut self, group: &str, option: &str) -> Self {
        let opt = ModOption {
            name: option.into(),
            description: Default::default(),
            path: PathBuf::from(format!("{group}/{option}")),
            requires: vec![],
        };
        self.options.push(opt.clone());
        self.meta
            .options
            .push(OptionGroup::Exclusive(ExclusiveOptionGroup {
                name: group.into(),
                description: Default::default(),
                required: true,
                default: None,
                options: vec![opt],
            }));
        self
    }

    /// Writes the mod as a UKMM zip.
    fn write(self, dir: &Path, file_name: &str) -> PathBuf {
        // The manifest is built through YAML so the string types line up with
        // the ones the mod format uses.
        let mut manifest_yaml = String::from("content:\n");
        for file in &self.files {
            manifest_yaml.push_str(&format!("  - {file}\n"));
        }
        manifest_yaml.push_str("aoc: []\n");
        let manifest: Manifest = serde_yaml::from_str(&manifest_yaml).unwrap();
        let path = dir.join(file_name);
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for (entry, body) in [
            ("meta.yml", serde_yaml::to_string(&self.meta).unwrap()),
            ("manifest.yml", serde_yaml::to_string(&manifest).unwrap()),
        ] {
            zip.start_file(entry, opts).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        for file in &self.files {
            zip.start_file(file.as_str(), opts).unwrap();
            zip.write_all(b"test content").unwrap();
        }
        for option in &self.options {
            let entry = option.manifest_path();
            zip.start_file(entry.to_str().unwrap(), opts).unwrap();
            zip.write_all(
                serde_yaml::to_string(&Manifest::default())
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        }
        zip.finish().unwrap();
        path
    }
}

/// A legacy mod: content files, but no metadata.
fn legacy_zip(dir: &Path, file_name: &str) -> PathBuf {
    let path = dir.join(file_name);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("content/legacy.txt", opts).unwrap();
    zip.write_all(b"legacy").unwrap();
    zip.finish().unwrap();
    path
}

fn temp_session() -> (tempfile::TempDir, Session) {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings::load_from(&dir.path().join("config").join("settings.yml"));
    // Persist the settings so `Reload` re-reads this temp file instead of
    // falling back to the real user config.
    settings.read().save().unwrap();
    let core = Manager::init_with(settings).unwrap();
    let session = Session::open(Arc::new(core)).unwrap();
    (dir, session)
}

// ----------------------------------------------------------------- helpers

/// Dispatches a command and collects the events up to its `JobDone`.
fn run(
    session: &Session,
    rx: &flume::Receiver<Event>,
    cmd: Command,
) -> (Vec<Event>, Result<(), AppError>) {
    let job = session.dispatch(cmd);
    let mut events = vec![];
    loop {
        let event = rx
            .recv_timeout(Duration::from_secs(30))
            .expect("session event");
        let done = matches!(&event, Event::JobDone { job: j, .. } if *j == job);
        events.push(event);
        if done {
            break;
        }
    }
    let result = events
        .iter()
        .find_map(|event| {
            match event {
                Event::JobDone { result, .. } => Some(result.clone()),
                _ => None,
            }
        })
        .expect("JobDone event");
    (events, result)
}

fn inspections(events: &[Event]) -> Vec<&Inspection> {
    events
        .iter()
        .filter_map(|event| {
            match event {
                Event::Inspection(inspection) => Some(inspection),
                _ => None,
            }
        })
        .collect()
}

fn ready_candidate(events: &[Event]) -> crate::session::InstallCandidate {
    inspections(events)
        .iter()
        .find_map(|inspection| {
            match inspection {
                Inspection::Ready(candidate) => Some(candidate.clone()),
                _ => None,
            }
        })
        .expect("Ready inspection")
}

/// Opens a mod and installs it, returning its id.
fn install(session: &Session, rx: &flume::Receiver<Event>, path: &Path) -> ModId {
    let (events, result) = run(session, rx, Command::Open {
        path: path.to_path_buf(),
        meta: None,
    });
    result.expect("open succeeds");
    let candidate = ready_candidate(&events);
    let (events, result) = run(session, rx, Command::Install(candidate));
    result.expect("install succeeds");
    events
        .iter()
        .find_map(|event| {
            match event {
                Event::Installed { id } => Some(*id),
                _ => None,
            }
        })
        .expect("Installed event")
}

fn profile_file(session: &Session, profile: &str) -> PathBuf {
    session
        .core()
        .settings()
        .profiles_dir()
        .join(profile)
        .join("profile.yml")
}

fn snapshot(session: &Session) -> Arc<Snapshot> {
    session.snapshot()
}

// ------------------------------------------------------------------- tests

#[test]
fn install_publishes_the_mod_and_stages_its_files() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");

    let id = install(&session, &rx, &path);

    let snap = snapshot(&session);
    assert_eq!(snap.entries.len(), 1);
    assert_eq!(snap.entries[0].id, id);
    assert_eq!(snap.entries[0].meta.name, "Test Mod");
    assert!(snap.entries[0].enabled);
    assert_eq!(snap.pending.files(), 1);
    assert!(snap.busy.is_none());
    assert!(profile_file(&session, "Default").exists());
}

#[test]
fn set_enabled_flips_the_entry_and_saves_the_profile() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    let id = install(&session, &rx, &path);

    let revision = snapshot(&session).revision;
    let (_, result) = run(&session, &rx, Command::SetEnabled {
        ids:     vec![id],
        enabled: false,
    });
    result.expect("set enabled succeeds");

    let snap = snapshot(&session);
    assert_eq!(snap.revision, revision + 1);
    assert!(!snap.entries[0].enabled);
    // Saved right away: the profile on disk has the new state (§22.6).
    let saved = std::fs::read_to_string(profile_file(&session, "Default")).unwrap();
    assert!(saved.contains("enabled: false"), "profile.yml: {saved}");
}

#[test]
fn move_to_changes_the_load_order_and_saves_it() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let first = Fixture::new("First")
        .with_file("content/a.txt")
        .write(dir.path(), "first.zip");
    let second = Fixture::new("Second")
        .with_file("content/b.txt")
        .write(dir.path(), "second.zip");
    let first_id = install(&session, &rx, &first);
    let second_id = install(&session, &rx, &second);

    let (_, result) = run(&session, &rx, Command::MoveTo {
        ids:   vec![second_id],
        index: 0,
    });
    result.expect("move succeeds");

    let snap = snapshot(&session);
    assert_eq!(snap.entries[0].id, second_id);
    assert_eq!(snap.entries[1].id, first_id);
    let saved = std::fs::read_to_string(profile_file(&session, "Default")).unwrap();
    let profile: uk_manager::mods::Profile = serde_yaml::from_str(&saved).unwrap();
    assert_eq!(
        profile.load_order().clone(),
        vec![second_id.0, first_id.0],
        "profile.yml: {saved}"
    );
}

#[test]
fn apply_without_a_dump_fails_and_keeps_the_pending_files() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    install(&session, &rx, &path);
    assert_eq!(snapshot(&session).pending.files(), 1);

    let (_, result) = run(&session, &rx, Command::Apply);
    let error = result.expect_err("apply without a dump fails");
    assert!(matches!(error, AppError::Other(_)), "{error:?}");
    // The staged files are not lost by the failed merge.
    assert_eq!(snapshot(&session).pending.files(), 1);
}

#[test]
fn cancel_pending_drops_the_staged_files() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    install(&session, &rx, &path);

    let (_, result) = run(&session, &rx, Command::CancelPending);
    result.expect("cancel succeeds");
    assert_eq!(snapshot(&session).pending.files(), 0);
}

#[test]
fn uninstall_removes_the_mod_and_saves_the_profile() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    let id = install(&session, &rx, &path);

    let (events, result) = run(&session, &rx, Command::Uninstall(vec![id]));
    result.expect("uninstall succeeds");
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Removed { ids } if ids == &vec![id]))
    );
    assert!(snapshot(&session).entries.is_empty());
    let saved = std::fs::read_to_string(profile_file(&session, "Default")).unwrap();
    assert!(!saved.contains("Test Mod"), "profile.yml: {saved}");
}

#[test]
fn open_reports_a_mod_without_metadata() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = legacy_zip(dir.path(), "legacy.zip");

    let (events, result) = run(&session, &rx, Command::Open {
        path: path.clone(),
        meta: None,
    });
    result.expect("open succeeds");
    let inspection = inspections(&events);
    assert!(
        inspection.iter().any(|i| {
            matches!(
                i,
                Inspection::NeedsMeta { path: p, .. } if p == &path
            )
        }),
        "expected NeedsMeta, got {inspection:?}"
    );
}

#[test]
fn open_reports_a_mod_for_the_wrong_platform() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Switch Mod")
        .with_platform(ModPlatform::Specific(Endian::Little))
        .write(dir.path(), "switch.zip");

    let (events, result) = run(&session, &rx, Command::Open {
        path: path.clone(),
        meta: None,
    });
    result.expect("open succeeds");
    let inspection = inspections(&events);
    assert!(
        inspection.iter().any(|i| matches!(
            i,
            Inspection::WrongPlatform { mod_is, current } if mod_is == "Switch" && current == "Wii U"
        )),
        "expected WrongPlatform, got {inspection:?}"
    );
}

#[test]
fn open_reports_missing_master_mods_without_blocking() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Dependent Mod")
        .with_masters(4242, "Master Mod", "1.0.0")
        .write(dir.path(), "dependent.zip");

    let (events, result) = run(&session, &rx, Command::Open {
        path: path.clone(),
        meta: None,
    });
    result.expect("open succeeds");
    let inspection = inspections(&events);
    assert!(
        inspection.iter().any(|i| {
            matches!(
                i,
                Inspection::MissingMasters(masters)
                    if masters.len() == 1 && masters[0].name == "Master Mod"
            )
        }),
        "expected MissingMasters, got {inspection:?}"
    );
    // Missing masters do not stop the install.
    assert!(
        inspection.iter().any(|i| matches!(i, Inspection::Ready(_))),
        "expected Ready, got {inspection:?}"
    );
}

#[test]
fn options_prompt_gates_required_groups() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Optional Mod")
        .with_required_option("group", "a")
        .write(dir.path(), "optional.zip");

    let (events, result) = run(&session, &rx, Command::Open {
        path: path.clone(),
        meta: None,
    });
    result.expect("open succeeds");
    let prompt = inspections(&events)
        .iter()
        .find_map(|inspection| {
            match inspection {
                Inspection::NeedsOptions(prompt) => Some(prompt.clone()),
                _ => None,
            }
        })
        .expect("NeedsOptions inspection");
    assert_eq!(prompt.groups().len(), 1);

    // A required group without a pick cannot be resolved.
    assert!(prompt.clone().resolve(vec![]).is_none());

    let pick = prompt.groups()[0].options()[0].clone();
    let candidate = prompt
        .clone()
        .resolve(vec![pick])
        .expect("required group is satisfied");
    let (_, result) = run(&session, &rx, Command::Install(candidate));
    result.expect("install succeeds");
    assert_eq!(snapshot(&session).entries.len(), 1);
}

#[test]
fn install_batch_reports_progress_and_aggregates_failures() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let good = Fixture::new("Good Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "good.zip");
    let junk = dir.path().join("junk.zip");
    std::fs::write(&junk, b"not a zip file").unwrap();

    let (events, result) = run(&session, &rx, Command::InstallBatch {
        paths:   vec![good.clone(), junk.clone()],
        replace: None,
    });
    // The good mod is installed, the junk file is reported as an error.
    assert!(result.is_err(), "the junk file must fail the batch");
    let installed = events
        .iter()
        .filter(|event| matches!(event, Event::Installed { .. }))
        .count();
    assert_eq!(installed, 1);
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Progress { .. })),
        "the batch must report progress"
    );
    // The queue is empty again (§22.36).
    assert_eq!(snapshot(&session).installing, 0);
    assert!(snapshot(&session).busy.is_none());
    assert_eq!(snapshot(&session).entries.len(), 1);
}

#[test]
fn every_dispatched_command_finishes_with_a_job_done_event() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    let id = install(&session, &rx, &path);

    // Fire a batch of commands without waiting in between (§22.8).
    for enabled in [false, true, false] {
        session.dispatch(Command::SetEnabled {
            ids: vec![id],
            enabled,
        });
    }
    session.dispatch(Command::CancelPending);

    let mut done = 0;
    let deadline = Duration::from_secs(30);
    while done < 4 {
        let event = rx.recv_timeout(deadline).expect("session event");
        if matches!(event, Event::JobDone { .. }) {
            done += 1;
        }
    }
    assert_eq!(done, 4);
}

#[test]
fn profiles_keep_their_own_pending_files() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let other = ProfileName::parse("Other").unwrap();

    let (_, result) = run(&session, &rx, Command::CreateProfile(other.clone()));
    result.expect("create profile succeeds");
    // A new profile starts empty — no fallback to another profile's files.
    assert_eq!(snapshot(&session).profile, other);
    assert_eq!(snapshot(&session).pending.files(), 0);
    assert!(snapshot(&session).entries.is_empty());

    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    let id = install(&session, &rx, &path);
    assert_eq!(snapshot(&session).pending.files(), 1);

    // Switching back shows the other profile's state.
    let (_, result) = run(
        &session,
        &rx,
        Command::SwitchProfile(ProfileName::parse("Default").unwrap()),
    );
    result.expect("switch succeeds");
    assert_eq!(snapshot(&session).profile.as_str(), "Default");
    assert!(snapshot(&session).entries.is_empty());
    assert_eq!(snapshot(&session).pending.files(), 0);

    // The staged files are still there for the profile they belong to.
    let (_, result) = run(&session, &rx, Command::SwitchProfile(other.clone()));
    result.expect("switch succeeds");
    assert_eq!(snapshot(&session).entries.len(), 1);
    assert_eq!(snapshot(&session).pending.files(), 1);
    assert_eq!(snapshot(&session).entries[0].id, id);
}

#[test]
fn add_to_profile_saves_the_target_profile() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let other = ProfileName::parse("Other").unwrap();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    let id = install(&session, &rx, &path);

    let (_, result) = run(&session, &rx, Command::CreateProfile(other.clone()));
    result.expect("create profile succeeds");
    // Back to the profile the mod lives in.
    let (_, result) = run(
        &session,
        &rx,
        Command::SwitchProfile(ProfileName::parse("Default").unwrap()),
    );
    result.expect("switch succeeds");
    let (_, result) = run(&session, &rx, Command::AddToProfile {
        ids:     vec![id],
        profile: other.clone(),
    });
    result.expect("add to profile succeeds");

    // The target profile is saved right away (§22.31).
    let saved = std::fs::read_to_string(profile_file(&session, "Other")).unwrap();
    assert!(saved.contains("Test Mod"), "profile.yml: {saved}");
}

#[test]
fn update_from_path_saves_the_profile() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let old = Fixture::new("Test Mod")
        .with_version("1.0.0")
        .with_file("content/a.txt")
        .write(dir.path(), "old.zip");
    let id = install(&session, &rx, &old);

    let new = Fixture::new("Test Mod")
        .with_version("2.0.0")
        .with_file("content/a.txt")
        .write(dir.path(), "new.zip");
    let (events, result) = run(&session, &rx, Command::UpdateFromPath {
        id,
        path: new.clone(),
    });
    result.expect("update succeeds");
    let candidate = ready_candidate(&events);
    assert_eq!(candidate.replace_target(), Some(id));
    let (_, result) = run(&session, &rx, Command::Install(candidate));
    result.expect("update install succeeds");

    // The replacement is saved right away (§22.38).
    let saved = std::fs::read_to_string(profile_file(&session, "Default")).unwrap();
    assert!(saved.contains("2.0.0"), "profile.yml: {saved}");
    assert_eq!(snapshot(&session).entries[0].meta.version, "2.0.0");
}

#[test]
fn renaming_a_profile_moves_its_directory_and_pending_files() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");
    install(&session, &rx, &path);

    let (_, result) = run(&session, &rx, Command::RenameProfile {
        from: ProfileName::parse("Default").unwrap(),
        to:   ProfileName::parse("Renamed").unwrap(),
    });
    result.expect("rename succeeds");

    let profiles_dir = session.core().settings().profiles_dir();
    assert!(profiles_dir.join("Renamed").is_dir());
    assert!(!profiles_dir.join("Default").exists());
    assert_eq!(snapshot(&session).profile.as_str(), "Renamed");
    assert_eq!(snapshot(&session).pending.files(), 1);
}

#[test]
fn the_active_profile_cannot_be_deleted() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    let (_, result) = run(
        &session,
        &rx,
        Command::DeleteProfile(ProfileName::parse("Default").unwrap()),
    );
    let error = result.expect_err("deleting the active profile fails");
    assert!(error.to_string().contains("active profile"), "{error}");
    assert!(
        session
            .core()
            .settings()
            .profiles_dir()
            .join("Default")
            .is_dir()
    );
}

#[test]
fn deleting_a_profile_removes_its_directory() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    let other = ProfileName::parse("Other").unwrap();
    let (_, result) = run(&session, &rx, Command::CreateProfile(other.clone()));
    result.expect("create profile succeeds");
    // Switch away, so the profile is not the active one any more.
    let (_, result) = run(
        &session,
        &rx,
        Command::SwitchProfile(ProfileName::parse("Default").unwrap()),
    );
    result.expect("switch succeeds");
    let (_, result) = run(&session, &rx, Command::DeleteProfile(other));
    result.expect("delete succeeds");
    assert!(
        !session
            .core()
            .settings()
            .profiles_dir()
            .join("Other")
            .exists()
    );
}

#[test]
fn deleting_a_profile_removes_orphaned_mod_files() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let mod_zip = Fixture::new("Orphan Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "orphan.zip");
    let id = install(&session, &rx, &mod_zip);
    // The stored copy of the mod (name is sanitised by the core).
    let stored = std::fs::read_dir(session.core().settings().mods_dir())
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .find(|p| p.extension().is_some_and(|e| e == "zip"))
        .expect("mod file stored");
    assert!(stored.exists(), "mod file stored at {}", stored.display());

    // Copy the mod into another profile, then delete that profile: the file
    // stays, because the active profile still uses it.
    let other = ProfileName::parse("Other").unwrap();
    let (_, result) = run(&session, &rx, Command::CreateProfile(other.clone()));
    result.expect("create profile succeeds");
    let (_, result) = run(&session, &rx, Command::SwitchProfile(other.clone()));
    result.expect("switch succeeds");
    let (_, result) = run(
        &session,
        &rx,
        Command::SwitchProfile(ProfileName::parse("Default").unwrap()),
    );
    result.expect("switch back succeeds");
    let (_, result) = run(&session, &rx, Command::AddToProfile {
        ids:     vec![id],
        profile: other.clone(),
    });
    result.expect("add to profile succeeds");
    let (_, result) = run(&session, &rx, Command::DeleteProfile(other));
    result.expect("delete succeeds");
    assert!(stored.exists(), "still used by the active profile");

    // Uninstall from the active profile and delete the last referencing
    // profile: now the file is orphaned and must go.
    let other2 = ProfileName::parse("Other2").unwrap();
    let (_, result) = run(&session, &rx, Command::CreateProfile(other2.clone()));
    result.expect("create second profile succeeds");
    let (_, result) = run(&session, &rx, Command::SwitchProfile(other2.clone()));
    result.expect("switch succeeds");
    let (_, result) = run(
        &session,
        &rx,
        Command::SwitchProfile(ProfileName::parse("Default").unwrap()),
    );
    result.expect("switch back succeeds");
    let (_, result) = run(&session, &rx, Command::AddToProfile {
        ids:     vec![id],
        profile: other2.clone(),
    });
    result.expect("add to second profile succeeds");
    let (_, result) = run(&session, &rx, Command::Uninstall(vec![id]));
    result.expect("uninstall succeeds");
    assert!(stored.exists(), "Other2 still references the file");
    let (_, result) = run(&session, &rx, Command::DeleteProfile(other2));
    result.expect("delete succeeds");
    assert!(!stored.exists(), "orphaned mod file removed");
}

#[test]
fn one_click_job_has_a_proper_busy_lifecycle() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    // An empty URL fails before any network access; what matters is the job
    // lifecycle: JobStarted → JobDone(Err), and busy cleared afterwards
    // (GUI_REF §22.26 — no stray flag).
    let (events, result) = run(&session, &rx, Command::OneClick(String::new()));
    assert!(result.is_err());
    assert!(events.iter().any(|e| matches!(e, Event::JobStarted(_))));
    let snap = snapshot(&session);
    assert!(snap.busy.is_none());
    assert!(snap.progress.is_none());
}

#[test]
fn reload_keeps_session_state_consistent() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let mod_zip = Fixture::new("Reload Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "reload.zip");
    let id = install(&session, &rx, &mod_zip);

    let (events, result) = run(&session, &rx, Command::Reload);
    result.expect("reload succeeds");
    assert!(events.iter().any(|e| matches!(e, Event::SettingsChanged)));
    assert!(events.iter().any(|e| matches!(e, Event::ProfilesChanged)));
    let snap = snapshot(&session);
    assert_eq!(snap.entries.len(), 1);
    assert_eq!(snap.entries[0].id, id);
    assert_eq!(snap.profile.as_str(), "Default");
}

#[test]
fn every_subscriber_sees_every_event() {
    let (dir, session) = temp_session();
    let first = session.subscribe();
    let second = session.subscribe();
    let path = Fixture::new("Test Mod")
        .with_file("content/a.txt")
        .write(dir.path(), "mod.zip");

    let (_, result) = run(&session, &first, Command::Open {
        path: path.clone(),
        meta: None,
    });
    result.expect("open succeeds");

    // The second subscriber gets the same events — they are broadcast, not
    // shared between receivers.
    let mut events = vec![];
    loop {
        let event = second
            .recv_timeout(Duration::from_secs(30))
            .expect("second subscriber sees the events");
        let done = matches!(event, Event::JobDone { .. });
        events.push(event);
        if done {
            break;
        }
    }
    assert!(
        events
            .iter()
            .any(|event| matches!(event, Event::Inspection(_))),
        "the second subscriber sees the inspection: {events:?}"
    );
}

#[test]
fn the_snapshot_is_unchanged_while_no_job_runs() {
    let (_dir, session) = temp_session();
    let before = snapshot(&session);
    assert!(before.busy.is_none());
    assert!(before.progress.is_none());
    assert_eq!(before.revision, 0);
    assert_eq!(before.profile.as_str(), "Default");
    assert!(before.entries.is_empty());
    assert!(before.pending.is_empty());
}

#[test]
fn display_rows_map_back_to_load_order_ids() {
    let entries = vec![
        entry(ModId(10), "Charlie", true),
        entry(ModId(20), "Alpha", false),
        entry(ModId(30), "Bravo", true),
    ];
    // Sorting by name gives display order…
    let visible = sort_entries(&entries, Sort::Name, true);
    assert_eq!(visible, vec![ModId(10), ModId(30), ModId(20)]);
    let filtered = filter_entries(&visible, &entries, "a");
    assert_eq!(filtered, visible);
    // …but commands use load-order indices (§22.1).
    assert_eq!(load_index_of(&entries, ModId(30)), Some(2));
    assert_eq!(load_index_of(&entries, ModId(10)), Some(0));

    // Moving a multi-selection moves exactly those mods.
    let mut order: Vec<ModId> = entries.iter().map(|e| e.id).collect();
    move_ids(&mut order, &[ModId(20), ModId(30)], 0, |id| *id);
    assert_eq!(order, vec![ModId(20), ModId(30), ModId(10)]);
}

#[test]
fn update_from_path_replaces_the_installed_file() {
    let (dir, session) = temp_session();
    let rx = session.subscribe();
    let old = Fixture::new("Test Mod")
        .with_version("1.0.0")
        .with_file("content/a.txt")
        .write(dir.path(), "old.zip");
    let id = install(&session, &rx, &old);

    let new = Fixture::new("Test Mod")
        .with_version("2.0.0")
        .with_file("content/b.txt")
        .write(dir.path(), "new.zip");
    let (events, result) = run(&session, &rx, Command::UpdateFromPath {
        id,
        path: new.clone(),
    });
    result.expect("update succeeds");
    let candidate = ready_candidate(&events);
    let (_, result) = run(&session, &rx, Command::Install(candidate));
    result.expect("update install succeeds");

    // The installed file is the new one, not just the new metadata.
    let installed = snapshot(&session).entries[0].path.clone();
    assert!(zip_entries(&installed).contains(&"content/b.txt".to_string()));
    assert!(!zip_entries(&installed).contains(&"content/a.txt".to_string()));
}

#[test]
fn switching_to_a_profile_created_outside_ukmm_works() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    // A profile directory that UKMM has never seen.
    let profiles_dir = session.core().settings().profiles_dir();
    let outside = profiles_dir.join("Outside");
    std::fs::create_dir_all(&outside).unwrap();
    std::fs::write(
        outside.join("profile.yml"),
        serde_yaml::to_string(&uk_manager::mods::Profile::default()).unwrap(),
    )
    .unwrap();

    let (_, result) = run(
        &session,
        &rx,
        Command::SwitchProfile(ProfileName::parse("Outside").unwrap()),
    );
    result.expect("switch succeeds");
    assert_eq!(snapshot(&session).profile.as_str(), "Outside");
    assert!(snapshot(&session).entries.is_empty());
}

#[test]
fn deleting_a_profile_created_outside_ukmm_works() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    // A profile directory the core never loaded (it appeared after startup):
    // deleting it must not panic in `get_profile`.
    let outside = session.core().settings().profiles_dir().join("Outside");
    std::fs::create_dir_all(&outside).unwrap();

    let (_, result) = run(
        &session,
        &rx,
        Command::DeleteProfile(ProfileName::parse("Outside").unwrap()),
    );
    result.expect("delete succeeds");
    assert!(!outside.exists());
}

#[test]
fn a_panicking_job_yields_job_done_err_and_the_session_survives() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    // A profile directory whose profile.yml cannot be parsed: loading it
    // panics inside the core, which the worker must contain (plan §6.9).
    let broken = session.core().settings().profiles_dir().join("Broken");
    std::fs::create_dir_all(&broken).unwrap();
    std::fs::write(broken.join("profile.yml"), "not: [valid").unwrap();

    let (events, result) = run(
        &session,
        &rx,
        Command::DeleteProfile(ProfileName::parse("Broken").unwrap()),
    );
    let error = result.expect_err("panicking job yields JobDone(Err)");
    assert!(matches!(error, AppError::Other(_)), "{error}");
    assert!(events.iter().any(|e| matches!(e, Event::Error(_))));

    // The worker caught the panic: the session still answers commands.
    let (_, result) = run(&session, &rx, Command::CancelPending);
    result.expect("session responsive after a panic");
    assert!(snapshot(&session).busy.is_none());
}

#[test]
fn a_background_job_runs_without_setting_busy() {
    let (_dir, session) = temp_session();
    let rx = session.subscribe();
    // OneClick with an empty URL fails fast, but a blocking job emits a
    // Progress event first (the handler reads `busy` for progress); a
    // background job must not.
    let job = session.dispatch_background(Command::OneClick(String::new()));
    let mut saw_progress = false;
    loop {
        let event = rx
            .recv_timeout(Duration::from_secs(30))
            .expect("session event");
        if matches!(&event, Event::Progress { .. }) {
            saw_progress = true;
        }
        if matches!(&event, Event::JobDone { job: j, .. } if *j == job) {
            break;
        }
    }
    assert!(!saw_progress, "background jobs do not report progress");
    assert!(snapshot(&session).busy.is_none());

    // The same command as a blocking job does report progress.
    let (events, result) = run(&session, &rx, Command::OneClick(String::new()));
    assert!(result.is_err());
    assert!(events.iter().any(|e| matches!(e, Event::Progress { .. })));
}

/// The names of the files in a zip archive.
fn zip_entries(path: &Path) -> Vec<String> {
    let file = std::fs::File::open(path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    (0..archive.len())
        .map(|index| archive.by_index(index).unwrap().name().unwrap().to_string())
        .collect()
}

fn entry(id: ModId, name: &str, enabled: bool) -> ModEntry {
    let mut meta = Meta::blank(ModPlatform::Universal);
    meta.name = name.into();
    ModEntry {
        id,
        meta,
        enabled,
        options: vec![],
        path: PathBuf::from(format!("/mods/{name}.zip")),
    }
}
