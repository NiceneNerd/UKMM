use std::{io::BufReader, path::Path};

use anyhow::Context;
use fs_err as fs;

/// Content folder names that mark an archive/directory as a BotW mod.
const MOD_ROOTS: &[&str] = &["content", "aoc", "01007EF00011E000", "01007EF00011F001"];

/// BNP archives additionally keep their logs at the root.
const BNP_ROOTS: &[&str] = &[
    "content",
    "aoc",
    "01007EF00011E000",
    "01007EF00011F001",
    "logs",
];

/// True if any `/`-separated segment of an archive entry name is one of
/// `roots`. This matches explicit directory entries (`content/`), plain files
/// inside them (`content/a.txt`, including archives written without directory
/// entries), and nested roots (`MyMod/content/…`).
fn has_root_segment(name: &str, roots: &[&str]) -> bool {
    name.split('/').any(|seg| roots.contains(&seg))
}

/// True if the final `/`-separated segment of an archive entry name is `file`.
fn has_file_segment(name: &str, file: &str) -> bool {
    name.rsplit('/').next() == Some(file)
}

pub fn is_probably_a_bnp(path: &Path) -> bool {
    match path
        .extension()
        .and_then(|e| e.to_str().map(|e| e.to_lowercase()))
        .unwrap_or_default()
        .as_str()
    {
        "" => {
            let is_a_mod = BNP_ROOTS.iter().any(|&root| path.join(root).exists());
            let has_meta = path.join("info.json").exists();
            is_a_mod && has_meta
        }
        "bnp" => {
            match sevenz_rust::Archive::open(path) {
                Ok(sz) => {
                    let mut is_a_mod = false;
                    let mut has_meta = false;
                    for entry in sz.files.iter() {
                        is_a_mod = is_a_mod || has_root_segment(&entry.name, BNP_ROOTS);
                        has_meta = has_meta || has_file_segment(&entry.name, "info.json");
                        if is_a_mod && has_meta {
                            break;
                        }
                    }
                    is_a_mod && has_meta
                }
                Err(_) => false,
            }
        }
        "json" => {
            let is_a_mod = BNP_ROOTS.iter().any(|&root| {
                path.parent()
                    .map(|p| p.join(root).exists())
                    .unwrap_or(false)
            });
            let has_meta = path.file_name().map(|f| f == "info.json").unwrap_or(false);
            is_a_mod && has_meta
        }
        _ => false,
    }
}

pub fn is_probably_a_mod_and_has_meta(path: &Path) -> (bool, bool) {
    if path
        .file_name()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default()
        == "rules.txt"
    {
        return (true, true);
    }
    if path.is_dir() {
        let is_a_mod = MOD_ROOTS.iter().any(|&root| path.join(root).exists());
        let has_meta = path.join("rules.txt").exists();
        return (is_a_mod, has_meta);
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str().map(|e| e.to_lowercase()))
        .unwrap_or_default();
    if ext != "zip" && ext != "7z" && ext != "rar" {
        (false, false)
    } else if ext == "rar" {
        match unrar::Archive::new(path).open_for_listing() {
            Ok(mut rar) => {
                let mut is_a_mod = false;
                let mut has_meta = false;
                while let Ok(Some(header)) = rar.read_header() {
                    let filename = header.entry().filename.to_string_lossy().replace('\\', "/");
                    is_a_mod = is_a_mod || has_root_segment(&filename, MOD_ROOTS);
                    has_meta = has_meta || has_file_segment(&filename, "rules.txt");
                    if is_a_mod && has_meta {
                        break;
                    }
                    rar = match header.skip() {
                        Ok(header) => header,
                        Err(_) => break,
                    }
                }
                (is_a_mod, has_meta)
            }
            Err(_) => (false, false),
        }
    } else if ext == "7z" {
        match sevenz_rust::Archive::open(path) {
            Ok(sz) => {
                let mut is_a_mod = false;
                let mut has_meta = false;
                for entry in sz.files.iter() {
                    is_a_mod = is_a_mod || has_root_segment(&entry.name, MOD_ROOTS);
                    has_meta = has_meta || has_file_segment(&entry.name, "rules.txt");
                    if is_a_mod && has_meta {
                        break;
                    }
                }
                (is_a_mod, has_meta)
            }
            Err(_) => (false, false),
        }
    } else {
        match fs::File::open(path)
            .context("")
            .and_then(|f| zip::ZipArchive::new(BufReader::new(f)).context(""))
        {
            Ok(zip) => {
                let mut is_a_mod = false;
                let mut has_meta = false;
                for name in zip.file_names().filter_map(Result::ok) {
                    log::info!("Checking file: {}", name);
                    is_a_mod = is_a_mod || has_root_segment(&name, MOD_ROOTS);
                    has_meta = has_meta || has_file_segment(&name, "rules.txt");
                    if is_a_mod && has_meta {
                        break;
                    }
                }
                (is_a_mod, has_meta)
            }
            Err(_) => (false, false),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    fn touch(path: &Path) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b"").unwrap();
    }

    #[test]
    fn rules_txt_is_a_mod_with_meta() {
        assert_eq!(
            is_probably_a_mod_and_has_meta(Path::new("anywhere/rules.txt")),
            (true, true)
        );
    }

    #[test]
    fn directory_mods_are_classified_by_contents() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(is_probably_a_mod_and_has_meta(dir.path()), (false, false));
        touch(&dir.path().join("content/a.txt"));
        assert_eq!(is_probably_a_mod_and_has_meta(dir.path()), (true, false));
        touch(&dir.path().join("rules.txt"));
        assert_eq!(is_probably_a_mod_and_has_meta(dir.path()), (true, true));
    }

    fn zip_with_entries(dir: &Path, dirs: &[&str], files: &[&str]) -> std::path::PathBuf {
        let path = dir.join("m.zip");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
        let opts = zip::write::SimpleFileOptions::default();
        for name in dirs {
            zip.add_directory(*name, opts).unwrap();
        }
        for name in files {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
        path
    }

    #[test]
    fn zip_mods_are_classified_by_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_with_entries(dir.path(), &["content/"], &["content/a.txt", "rules.txt"]);
        assert_eq!(is_probably_a_mod_and_has_meta(&path), (true, true));
    }

    /// Archives written without explicit directory entries must still be
    /// recognised — detection matches any path segment, not just `content/`.
    #[test]
    fn zip_mods_without_directory_entries_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_with_entries(dir.path(), &[], &["content/a.txt", "rules.txt"]);
        assert_eq!(is_probably_a_mod_and_has_meta(&path), (true, true));
    }

    #[test]
    fn zip_mods_with_nested_root_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_with_entries(dir.path(), &["MyMod/content/"], &["MyMod/rules.txt"]);
        assert_eq!(is_probably_a_mod_and_has_meta(&path), (true, true));
    }

    #[test]
    fn similarly_named_files_are_not_roots() {
        let dir = tempfile::tempdir().unwrap();
        let path = zip_with_entries(dir.path(), &[], &["mycontent/a.txt", "xrules.txt"]);
        assert_eq!(is_probably_a_mod_and_has_meta(&path), (false, false));
    }

    fn sz_with_entries(dir: &Path, name: &str, entries: &[(&str, &str)]) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut sz = sevenz_rust::SevenZWriter::create(&path).unwrap();
        for (entry, body) in entries {
            let src = dir.join("sz_src.tmp");
            std::fs::write(&src, body.as_bytes()).unwrap();
            let file = std::fs::File::open(&src).unwrap();
            sz.push_archive_entry(
                sevenz_rust::SevenZArchiveEntry::from_path(&src, entry.to_string()),
                Some(file),
            )
            .unwrap();
        }
        sz.finish().unwrap();
        path
    }

    #[test]
    fn bnp_7z_archives_are_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = sz_with_entries(dir.path(), "mod.bnp", &[
            ("content/Pack/foo.pack", "x"),
            ("info.json", "{}"),
        ]);
        assert!(is_probably_a_bnp(&path));
        assert_eq!(is_probably_a_mod_and_has_meta(&path), (false, false));
    }

    #[test]
    fn sevenz_mod_archives_are_detected_without_directory_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = sz_with_entries(dir.path(), "mod.7z", &[
            ("content/a.txt", "x"),
            ("rules.txt", "x"),
        ]);
        assert_eq!(is_probably_a_mod_and_has_meta(&path), (true, true));
        assert!(!is_probably_a_bnp(&path));
    }

    #[test]
    fn non_7z_bnp_is_not_detected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("fake.bnp");
        std::fs::write(&path, b"definitely not a 7z archive").unwrap();
        assert!(!is_probably_a_bnp(&path));
    }

    #[test]
    fn unsupported_extensions_are_not_mods() {
        assert_eq!(
            is_probably_a_mod_and_has_meta(Path::new("m.exe")),
            (false, false)
        );
        assert!(!is_probably_a_bnp(Path::new("m.exe")));
    }

    #[test]
    fn bnp_directory_needs_info_json() {
        let dir = tempfile::tempdir().unwrap();
        touch(&dir.path().join("content/a.txt"));
        assert!(!is_probably_a_bnp(dir.path()));
        touch(&dir.path().join("info.json"));
        assert!(is_probably_a_bnp(dir.path()));
    }
}
