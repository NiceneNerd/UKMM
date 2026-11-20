//! Smoke test for the CLI frontend: runs the real `ukmm` binary against a
//! fixture config (via `UKMM_CONFIG_DIR`) with a synthetic dump and generated
//! mod zips, and asserts the `--json` output shape. Merge/deploy against a
//! *real* game dump stays gated on `UKMM_TEST_DUMP` in `crates/uk-app/tests/e2e.rs`.

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command as Proc, Output},
    sync::Arc,
};

use uk_app::{
    model::{
        ExclusiveOptionGroup, Language, Manifest, Meta, ModOption, ModPlatform, OptionGroup,
        Platform, PlatformSettings, Settings,
    },
    testkit::{fake_dump, mod_zip_named},
};
use uk_reader::ResourceReader;

const BIN: &str = env!("CARGO_BIN_EXE_ukmm");

struct Fixture {
    _dir:   tempfile::TempDir,
    config: PathBuf,
}

/// A temp config dir with a settings.yml pointing at a synthetic unpacked
/// dump, plus two generated mod zips.
fn fixture() -> (Fixture, PathBuf, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::create_dir_all(&config).unwrap();
    let content = fake_dump(&dir.path().join("dump"));
    let settings = Settings::load_from(&config.join("settings.yml"));
    {
        let mut s = settings.write();
        s.current_mode = Platform::WiiU;
        s.wiiu_config = Some(PlatformSettings {
            language: Language::USen,
            profile: "Default".into(),
            dump: Arc::new(
                ResourceReader::from_unpacked_dirs(
                    Some(content),
                    None::<PathBuf>,
                    None::<PathBuf>,
                    uk_content::prelude::Endian::Big,
                )
                .unwrap(),
            ),
            deploy_config: None,
        });
        s.save().unwrap();
    }
    let a = mod_zip_named(dir.path(), "a.zip", "CLI Test Mod A");
    let b = mod_zip_named(dir.path(), "b.zip", "CLI Test Mod B");
    (Fixture { _dir: dir, config }, a, b)
}

fn ukmm(fix: &Fixture, args: &[&str]) -> Output {
    Proc::new(BIN)
        .args(args)
        .env("UKMM_CONFIG_DIR", &fix.config)
        .output()
        .expect("failed to run ukmm")
}

/// Every stdout line must be valid JSON with an `event` or `snapshot` key.
fn assert_jsonl(out: &Output, ctx: &str) {
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.trim().is_empty(),
        "expected JSONL output for {ctx}, got none; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    for line in stdout.lines() {
        let v: serde_json::Value =
            serde_json::from_str(line).unwrap_or_else(|e| panic!("bad JSONL line {line:?}: {e}"));
        assert!(
            v.get("event").is_some() || v.get("snapshot").is_some(),
            "line has neither event nor snapshot: {line}"
        );
    }
}

#[test]
fn json_install_enable_uninstall_profile_cycle() {
    let (fix, a, b) = fixture();
    let a = a.to_str().unwrap().to_string();
    let b = b.to_str().unwrap().to_string();

    // install a mod: JSONL on stdout, exit 0
    let out = ukmm(&fix, &["--json", "install", &a]);
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_jsonl(&out, "install a");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"Installed\""),
        "no Installed event:\n{stdout}"
    );

    // install the second mod into a new profile
    // `profile` is a positional (the `-p` short flag belongs to `--portable`)
    let out = ukmm(&fix, &["--json", "install", &b, "Second"]);
    assert!(
        out.status.success(),
        "install --profile failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_jsonl(&out, "install b --profile Second");

    // the new profile exists on disk and holds only mod b
    let profile_file = fix.config.join("data/wiiu/profiles/Second/profile.yml");
    let profile_text = std::fs::read_to_string(&profile_file).expect("profile.yml missing");
    assert!(profile_text.contains("CLI Test Mod B"), "{profile_text}");
    assert!(!profile_text.contains("CLI Test Mod A"), "{profile_text}");
    // ...and the current profile was left untouched (install-into-profile
    // switches there instead of copying)
    let default_file = fix.config.join("data/wiiu/profiles/Default/profile.yml");
    let default_text = std::fs::read_to_string(&default_file).expect("profile.yml missing");
    assert!(default_text.contains("CLI Test Mod A"), "{default_text}");
    assert!(!default_text.contains("CLI Test Mod B"), "{default_text}");

    // uninstall mod 1 (1-based index) from the current profile
    let out = ukmm(&fix, &["--json", "uninstall", "1"]);
    assert!(
        out.status.success(),
        "uninstall failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_jsonl(&out, "uninstall 1");

    // bad index fails with exit 1 (argument validation happens before any
    // command is dispatched, so there is no event stream)
    let out = ukmm(&fix, &["--json", "uninstall", "99"]);
    assert!(!out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("does not exist"),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // mode switch round-trips through settings.yml
    let out = ukmm(&fix, &["--json", "mode", "Switch"]);
    assert!(
        out.status.success(),
        "mode failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let settings_text = std::fs::read_to_string(fix.config.join("settings.yml")).unwrap();
    assert!(
        settings_text.contains("current_mode: Switch"),
        "{settings_text}"
    );
    let out = ukmm(&fix, &["--json", "mode", "WiiU"]);
    assert!(out.status.success());
}

#[test]
fn list_enable_disable_move_profiles() {
    let (fix, a, b) = fixture();
    let a = a.to_str().unwrap().to_string();
    let b = b.to_str().unwrap().to_string();
    for m in [&a, &b] {
        let out = ukmm(&fix, &["install", m]);
        assert!(
            out.status.success(),
            "install failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    // list shows both mods; --json list emits a snapshot line
    let out = ukmm(&fix, &["list"]);
    assert!(out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("CLI Test Mod A"), "{stdout}");
    assert!(stdout.contains("CLI Test Mod B"), "{stdout}");
    let out = ukmm(&fix, &["--json", "list"]);
    assert!(out.status.success());
    assert_jsonl(&out, "list --json");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("\"entries\""),
        "no entries in snapshot"
    );

    // disable/enable round-trip; each stages + merges in one invocation.
    // With --json, `disable 1` runs two jobs (SetEnabled, then Apply), both
    // Ok — without the auto-apply there would be only one JobDone.
    let out = ukmm(&fix, &["--json", "disable", "1"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_jsonl(&out, "disable --json");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let job_dones = stdout.matches("\"JobDone\"").count();
    assert_eq!(job_dones, 2, "expected SetEnabled + Apply jobs:\n{stdout}");
    assert!(!stdout.contains("\"Err\""), "job failed:\n{stdout}");
    let out = ukmm(&fix, &["list"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("[disabled]"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = ukmm(&fix, &["enable", "1"]);
    assert!(out.status.success());

    // move mod 2 to position 1
    let out = ukmm(&fix, &["move", "2", "1"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = ukmm(&fix, &["list"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    let pos_b = stdout.find("CLI Test Mod B").unwrap();
    let pos_a = stdout.find("CLI Test Mod A").unwrap();
    assert!(pos_b < pos_a, "move did not reorder:\n{stdout}");

    // profile lifecycle
    let out = ukmm(&fix, &["profile-create", "Second"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = ukmm(&fix, &["profile-switch", "Second"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = ukmm(&fix, &["list"]);
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Profile: Second"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = ukmm(&fix, &["profiles"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Default") && stdout.contains("Second"),
        "{stdout}"
    );
    // the active profile cannot be deleted — switch back first
    let out = ukmm(&fix, &["profile-switch", "Default"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = ukmm(&fix, &["profile-delete", "Second"]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = ukmm(&fix, &["profiles"]);
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("Second"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
}

#[test]
fn parse_meta_and_open_report_inspection() {
    let (fix, a, _b) = fixture();
    let a = a.to_str().unwrap().to_string();
    let meta_file = fix.config.parent().unwrap().join("meta.yml");
    let mut m = uk_app::model::Meta::blank(uk_app::model::ModPlatform::Universal);
    m.name = "Parsed Mod".into();
    std::fs::write(&meta_file, serde_yaml::to_string(&m).unwrap()).unwrap();
    let out = ukmm(&fix, &["--json", "parse-meta", meta_file.to_str().unwrap()]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_jsonl(&out, "parse-meta");
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("PackageMeta"),
        "no PackageMeta event"
    );

    // open reports Ready without installing
    let out = ukmm(&fix, &["open", &a]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stdout).contains("Ready to install"),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let out = ukmm(&fix, &["list"]);
    assert!(
        !String::from_utf8_lossy(&out.stdout).contains("CLI Test Mod A"),
        "open must not install"
    );
}

#[test]
fn json_package_emits_packaged_event() {
    let (fix, _a, _b) = fixture();
    let src = fix.config.parent().unwrap().join("src");
    // an empty content/ folder packages fine (non-mergeable files are rejected)
    std::fs::create_dir_all(src.join("content")).unwrap();
    let meta = fix.config.parent().unwrap().join("meta.yml");
    let out_zip = fix.config.parent().unwrap().join("out.zip");
    let mut m = uk_app::model::Meta::blank(uk_app::model::ModPlatform::Universal);
    m.name = "Packed Mod".into();
    std::fs::write(&meta, serde_yaml::to_string(&m).unwrap()).unwrap();
    let out = ukmm(&fix, &[
        "--json",
        "package",
        src.to_str().unwrap(),
        out_zip.to_str().unwrap(),
        meta.to_str().unwrap(),
    ]);
    assert!(
        out.status.success(),
        "package failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_jsonl(&out, "package");
    assert!(out_zip.exists(), "no output zip");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"Packaged\""),
        "no Packaged event:\n{stdout}"
    );
}

#[test]
fn json_unknown_dump_error_is_json_and_nonzero() {
    // A config with no dump configured: remerge must fail, exit 1, JSONL.
    let dir = tempfile::tempdir().unwrap();
    let config = dir.path().join("config");
    std::fs::create_dir_all(&config).unwrap();
    let fix = Fixture {
        _dir:   dir,
        config: config.clone(),
    };
    let out = ukmm(&fix, &["--json", "remerge"]);
    assert!(!out.status.success());
    assert_jsonl(&out, "remerge without dump");
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("JobDone"), "no JobDone event:\n{stdout}");
}

#[test]
fn malformed_cli_invocation_exits_2_not_silent_gui() {
    let (fix, a, _b) = fixture();
    let a = a.to_str().unwrap().to_string();
    // A subcommand with a bad flag must exit 2 with the parse error, not fall
    // through to the GUI (which would exit 0 on a headless machine).
    let out = ukmm(&fix, &["install", &a, "--profile", "Second"]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("unexpected flag"),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // An unknown subcommand likewise.
    let out = ukmm(&fix, &["boguscmd"]);
    assert_eq!(out.status.code(), Some(2));
}

/// A mod zip with one exclusive option group (paths `opt-a` / `opt-b`).
fn mod_zip_with_options(dir: &Path, file_name: &str) -> PathBuf {
    let mut meta = Meta::blank(ModPlatform::Universal);
    meta.name = "CLI Options Mod".into();
    meta.options = vec![OptionGroup::Exclusive(ExclusiveOptionGroup {
        name: "Texture".into(),
        description: "Pick a texture".into(),
        required: false,
        default: None,
        options: vec![
            ModOption {
                name: "A".into(),
                description: "".into(),
                path: "opt-a".into(),
                requires: vec![],
            },
            ModOption {
                name: "B".into(),
                description: "".into(),
                path: "opt-b".into(),
                requires: vec![],
            },
        ],
    })];
    let path = dir.join(file_name);
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
    for opt in ["opt-a", "opt-b"] {
        zip.start_file(format!("options/{opt}/manifest.yml"), opts)
            .unwrap();
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

/// A legacy mod zip: content but no meta.yml/manifest.yml.
fn legacy_zip(dir: &Path, file_name: &str) -> PathBuf {
    let path = dir.join(file_name);
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let opts = zip::write::SimpleFileOptions::default();
    zip.start_file("content/Map/MainField/A-1/A-1.00_Clustering.sblwp", opts)
        .unwrap();
    zip.write_all(b"legacy").unwrap();
    zip.finish().unwrap();
    path
}

#[test]
fn options_discovery_and_pick_validation() {
    let (fix, _a, _b) = fixture();
    let dir = fix.config.parent().unwrap().to_path_buf();
    let opt = mod_zip_with_options(&dir, "opt.zip");
    let opt = opt.to_str().unwrap().to_string();
    let out = ukmm(&fix, &["install", &opt]);
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // Discovery: no picks -> lists the group and its paths, changes nothing.
    let out = ukmm(&fix, &["options", "1"]);
    assert!(
        out.status.success(),
        "options failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("Texture"), "{stdout}");
    assert!(
        stdout.contains("opt-a") && stdout.contains("opt-b"),
        "{stdout}"
    );

    // Unknown pick -> error that lists the valid paths.
    let out = ukmm(&fix, &["options", "1", "bogus"]);
    assert!(!out.status.success(), "bogus pick should fail");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Unknown option path"), "{stderr}");
    assert!(stderr.contains("opt-a"), "{stderr}");

    // Valid pick -> applied and merged; discovery marks it selected.
    let out = ukmm(&fix, &["options", "1", "opt-b"]);
    assert!(
        out.status.success(),
        "options failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out = ukmm(&fix, &["options", "1"]);
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("* opt-b"), "{stdout}");
}

#[test]
fn update_from_legacy_mod_with_meta_replaces_in_place() {
    let (fix, a, _b) = fixture();
    let dir = fix.config.parent().unwrap().to_path_buf();
    let a = a.to_str().unwrap().to_string();
    let out = ukmm(&fix, &["install", &a]);
    assert!(
        out.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // A legacy archive (no meta.yml) plus an explicit --meta file.
    let legacy = legacy_zip(&dir, "update.zip");
    let legacy = legacy.to_str().unwrap().to_string();
    let mut m = Meta::blank(ModPlatform::Specific(uk_content::prelude::Endian::Big));
    m.name = "Updated Mod Name".into();
    m.version = "2.0.0".into();
    let meta_file = dir.join("new-meta.yml");
    std::fs::write(&meta_file, serde_yaml::to_string(&m).unwrap()).unwrap();
    let meta_file = meta_file.to_str().unwrap().to_string();

    let out = ukmm(&fix, &[
        "--json",
        "update-from",
        "1",
        &legacy,
        "--meta",
        &meta_file,
    ]);
    assert!(
        out.status.success(),
        "update-from failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("\"Updated\""),
        "no Updated event:\n{stdout}"
    );

    // The mod was replaced in place: still one entry, now with the new name.
    let out = ukmm(&fix, &["list"]);
    let list = String::from_utf8_lossy(&out.stdout);
    assert!(list.contains("Updated Mod Name"), "{list}");
    assert!(!list.contains("CLI Test Mod A"), "{list}");
    assert_eq!(
        list.lines().filter(|l| l.starts_with("1. ")).count(),
        1,
        "{list}"
    );
}
