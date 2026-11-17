//! Parsing and resolution of Cemu's `settings.xml` / `title_list_cache.xml`,
//! split from the filesystem effects in [`super::import_cemu_settings`] so the
//! XML logic can be tested against fixtures.

use std::path::{Path, PathBuf};

use anyhow_ext::{Context, Result, bail};
use fs_err as fs;
use join_str::jstr;
use uk_util::PathExt;

/// The locations involved in a Cemu import.
#[derive(Debug, Clone)]
pub struct CemuDirs {
    /// The Cemu installation/portable dir the user picked.
    pub root:     PathBuf,
    /// `<root>/portable`
    pub portable: PathBuf,
    /// OS config dir + `Cemu`
    pub config:   PathBuf,
    /// OS local data dir + `Cemu`
    pub local:    PathBuf,
}

impl CemuDirs {
    pub fn detect(root: &Path) -> Self {
        Self {
            root:     root.to_path_buf(),
            portable: root.join("portable"),
            config:   dirs2::config_dir().expect("No config dir").join("Cemu"),
            local:    dirs2::data_local_dir()
                .expect("No local data dir")
                .join("Cemu"),
        }
    }
}

/// Region suffixes of the Wii U BotW title IDs we accept (both cases).
pub const REGIONS: &[&str] = &[
    "101C9400", "101c9400", "101C9500", "101c9500", "101C9300", "101c9300",
];

/// Locates the Cemu `settings.xml`, preferring a portable install.
pub fn find_settings_xml(dirs: &CemuDirs) -> Result<PathBuf> {
    if let Some(path) = dirs.portable.join("settings.xml").exists_then() {
        return Ok(path);
    }
    #[cfg(windows)]
    if let Some(path) = dirs
        .root
        .join("settings.xml")
        .exists_then()
        .or_else(|| dirs.config.join("settings.xml").exists_then())
    {
        return Ok(path);
    }
    #[cfg(not(windows))]
    if let Some(path) = dirs.config.join("settings.xml").exists_then() {
        return Ok(path);
    }
    bail!("Could not find Cemu settings file. Please run Cemu at least once to generate it.")
}

/// Pure: extracts the `mlc_path` entry from `settings.xml` text.
pub fn parse_mlc_path(xml: &str) -> Option<PathBuf> {
    let tree = roxmltree::Document::parse(xml).ok()?;
    tree.descendants()
        .find_map(|n| {
            (n.tag_name().name() == "mlc_path").then(|| {
                n.text()
                    .and_then(|s| (!s.is_empty()).then(|| PathBuf::from(s)))
            })
        })
        .flatten()
}

/// Pure: scrapes the unpacked game's `content` dir from a `<path>` entry in
/// `settings.xml` that points at `U-King.rpx` (i.e. `<game root>/code/U-King.rpx`).
pub fn scrape_game_content_dir(xml: &str) -> Option<PathBuf> {
    xml.lines().map(str::trim).find_map(|line| {
        let inner = line.strip_prefix("<path>")?.strip_suffix("</path>")?;
        if !inner.contains("U-King.rpx") {
            return None;
        }
        let rpx = PathBuf::from(inner);
        let code = rpx.parent()?;
        let root = if code.file_name()? == "code" {
            code.parent()?
        } else {
            code
        };
        Some(root.join("content"))
    })
}

/// One entry of Cemu's `title_list_cache.xml`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TitleEntry {
    pub title_id: String,
    pub format:   u32,
    pub path:     PathBuf,
}

/// Pure: parses `title_list_cache.xml` into entries. Malformed `<title>` nodes
/// are an error, never a panic.
pub fn parse_title_list(xml: &str) -> Result<Vec<TitleEntry>> {
    let tree =
        roxmltree::Document::parse(xml).context("Failed to parse Cemu title cache: invalid XML")?;
    tree.descendants()
        .filter(|n| n.tag_name().name() == "title")
        .map(|n| {
            let title_id = n
                .attribute("titleId")
                .context("Invalid title cache entry: missing titleId")?
                .to_owned();
            let child_text = |tag: &str| -> Result<String> {
                n.descendants()
                    .find(|c| c.tag_name().name() == tag)
                    .and_then(|c| c.text().map(str::to_owned))
                    .with_context(|| format!("Invalid title cache entry {title_id}: missing {tag}"))
            };
            let format = child_text("format")?
                .parse::<u32>()
                .with_context(|| format!("Invalid title cache entry {title_id}: bad format"))?;
            let path = PathBuf::from(child_text("path")?);
            Ok(TitleEntry {
                title_id,
                format,
                path,
            })
        })
        .collect()
}

/// The dump locations resolved from Cemu settings.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DumpPaths {
    pub base: Option<PathBuf>,
    pub update: Option<PathBuf>,
    pub dlc: Option<PathBuf>,
    pub wua: Option<PathBuf>,
    pub encrypted: bool,
}

/// Resolves title-list entries against the filesystem: paths are canonicalized,
/// and relative paths are tried against `root`, `portable`, `local`, `config`
/// in that order. Entries for other titles/regions are ignored.
pub fn resolve_from_title_list(dirs: &CemuDirs, entries: &[TitleEntry]) -> DumpPaths {
    let mut out = DumpPaths::default();
    for entry in entries {
        let Some(region) = entry.title_id.get(8..) else {
            continue;
        };
        if !REGIONS.contains(&region) {
            continue;
        }
        let Some(dump_type) = entry.title_id.get(..8) else {
            continue;
        };
        let mut dump_path = entry
            .path
            .canonicalize()
            .unwrap_or_else(|_| entry.path.clone());
        if dump_path.is_relative() {
            let Some(resolved) = [
                dirs.root.as_path(),
                dirs.portable.as_path(),
                dirs.local.as_path(),
                dirs.config.as_path(),
            ]
            .into_iter()
            .map(|base| base.join(&entry.path))
            .find(|p| p.exists()) else {
                continue;
            };
            dump_path = resolved;
        }
        out.add(dump_type, entry.format, dump_path);
    }
    out
}

impl DumpPaths {
    fn add(&mut self, dump_type: &str, format: u32, dump_path: PathBuf) {
        match (dump_type, format) {
            ("00050000", 1) => self.base = Some(dump_path.join("content")),
            ("0005000c", 1) => self.dlc = Some(dump_path.join("content").join("0010")),
            ("0005000e", 1) => self.update = Some(dump_path.join("content")),
            ("00050000", 4) | ("0005000c", 4) | ("0005000e", 4) => self.encrypted = true,
            ("00050000", 3) => self.wua = Some(dump_path),
            _ => (),
        }
    }
}

/// Resolves an unpacked dump from the MLC path (`<mlc>/usr/title/…`), falling
/// back to the scraped `U-King.rpx` game path from `settings_xml`.
pub fn resolve_from_mlc(mlc: &Path, settings_xml: &str) -> DumpPaths {
    let title_path = mlc.join("usr/title");
    let base = REGIONS
        .iter()
        .find_map(|r| {
            let path = title_path.join(jstr!("00050000/{r}/content"));
            path.exists().then_some(path)
        })
        .or_else(|| scrape_game_content_dir(settings_xml));
    let update = REGIONS.iter().find_map(|r| {
        let path = title_path.join(jstr!("0005000e/{r}/content"));
        path.exists().then_some(path)
    });
    let dlc = REGIONS.iter().find_map(|r| {
        let path = title_path.join(jstr!("0005000c/{r}/content/0010"));
        path.exists().then_some(path)
    });
    DumpPaths {
        base,
        update,
        dlc,
        wua: None,
        encrypted: false,
    }
}

/// Finds the MLC dir: `mlc_path` from settings, else `mlc01` in the portable /
/// install / OS Cemu dirs.
pub fn find_mlc_dir(dirs: &CemuDirs, settings_xml: &str) -> Option<PathBuf> {
    parse_mlc_path(settings_xml)
        .or_else(|| dirs.portable.join("mlc01").exists_then())
        .or_else(|| {
            #[cfg(windows)]
            {
                dirs.root
                    .join("mlc01")
                    .exists_then()
                    .or_else(|| dirs.config.join("mlc01").exists_then())
            }
            #[cfg(not(windows))]
            {
                dirs.local.join("mlc01").exists_then()
            }
        })
}

/// Locates the title list cache, if Cemu has written one.
pub fn find_title_cache(dirs: &CemuDirs) -> Option<PathBuf> {
    dirs.config
        .join("title_list_cache.xml")
        .exists_then()
        .or_else(|| dirs.local.join("title_list_cache.xml").exists_then())
}

/// Resolves the game dump locations from Cemu settings XML, using the title
/// list cache when present and the MLC dir otherwise.
pub fn resolve_dump(dirs: &CemuDirs, settings_xml: &str) -> Result<DumpPaths> {
    if let Some(cache) = find_title_cache(dirs) {
        let text = fs::read_to_string(&cache).context("Failed to open Cemu title cache file")?;
        let entries = parse_title_list(&text)?;
        Ok(resolve_from_title_list(dirs, &entries))
    } else {
        let mlc = find_mlc_dir(dirs, settings_xml)
            .context("Could not find unpacked game dump from Cemu settings.")?;
        Ok(resolve_from_mlc(&mlc, settings_xml))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETTINGS_XML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cemu/settings.xml"
    ));
    const TITLE_CACHE_XML: &str = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/cemu/title_list_cache.xml"
    ));

    fn dirs(root: &Path) -> CemuDirs {
        CemuDirs {
            root:     root.to_path_buf(),
            portable: root.join("portable"),
            config:   root.join("config-cemu"),
            local:    root.join("local-cemu"),
        }
    }

    #[test]
    fn parses_mlc_path() {
        assert_eq!(
            parse_mlc_path(SETTINGS_XML).as_deref(),
            Some(Path::new("/cemu/mlc01"))
        );
        assert_eq!(parse_mlc_path("<settings/>"), None);
        assert_eq!(
            parse_mlc_path("<settings><mlc_path></mlc_path></settings>"),
            None
        );
        assert!(parse_mlc_path("not xml at all").is_none());
    }

    #[test]
    fn scrapes_game_content_dir_from_rpx_path() {
        let xml = "<settings><game_paths>\n  \
                   <path>/games/BotW/code/U-King.rpx</path>\n</game_paths></settings>";
        assert_eq!(
            scrape_game_content_dir(xml).as_deref(),
            Some(Path::new("/games/BotW/content"))
        );
        // lines without U-King.rpx and malformed lines are ignored, not panics
        assert_eq!(
            scrape_game_content_dir("<settings><path>/games/other</path></settings>"),
            None
        );
        assert_eq!(scrape_game_content_dir("<path>short</path>"), None);
        assert_eq!(scrape_game_content_dir("<path>"), None);
    }

    #[test]
    fn parses_title_cache_entries() {
        let entries = parse_title_list(TITLE_CACHE_XML).unwrap();
        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].title_id, "00050000101C9400");
        assert_eq!(entries[0].format, 1);
        assert_eq!(entries[0].path, PathBuf::from("/dumps/base"));
        assert_eq!(entries[3].title_id, "0005000e101c9300");
        assert_eq!(entries[3].format, 4);
    }

    #[test]
    fn malformed_title_cache_is_an_error_not_a_panic() {
        assert!(parse_title_list("<cache><title titleId=\"00050000101C9400\"/></cache>").is_err());
        assert!(
            parse_title_list(
                "<cache><title titleId=\"x\"><format>abc</format><path>/p</path></title></cache>"
            )
            .is_err()
        );
        assert!(parse_title_list("garbage{").is_err());
    }

    #[test]
    fn resolves_dump_from_title_list_with_regions_and_formats() {
        let dir = tempfile::tempdir().unwrap();
        let base = dir.path().join("base");
        let update = dir.path().join("update");
        let dlc = dir.path().join("dlc");
        std::fs::create_dir_all(&base).unwrap();
        std::fs::create_dir_all(&update).unwrap();
        std::fs::create_dir_all(&dlc).unwrap();
        let xml = format!(
            "<cache><title \
             titleId=\"00050000101C9400\"><format>1</format><path>{}</path></title><title \
             titleId=\"0005000e101C9500\"><format>1</format><path>{}</path></title><title \
             titleId=\"0005000c101c9300\"><format>1</format><path>{}</path></title><title \
             titleId=\"0005000010199900\"><format>1</format><path>{}</path></title></cache>",
            base.display(),
            update.display(),
            dlc.display(),
            dir.path().join("other").display(),
        );
        let entries = parse_title_list(&xml).unwrap();
        let resolved = resolve_from_title_list(&dirs(dir.path()), &entries);
        let base_canon = base.canonicalize().unwrap().join("content");
        let update_canon = update.canonicalize().unwrap().join("content");
        let dlc_canon = dlc.canonicalize().unwrap().join("content").join("0010");
        assert_eq!(resolved.base, Some(base_canon));
        assert_eq!(resolved.update, Some(update_canon));
        assert_eq!(resolved.dlc, Some(dlc_canon));
        assert!(!resolved.encrypted);
        assert!(resolved.wua.is_none());
    }

    #[test]
    fn format_4_marks_encrypted_and_format_3_is_a_wua() {
        let dir = tempfile::tempdir().unwrap();
        let enc = dir.path().join("enc");
        let wua = dir.path().join("game.wua");
        std::fs::create_dir_all(&enc).unwrap();
        std::fs::write(&wua, b"").unwrap();
        let xml = format!(
            "<cache><title \
             titleId=\"00050000101C9400\"><format>4</format><path>{}</path></title><title \
             titleId=\"00050000101C9400\"><format>3</format><path>{}</path></title></cache>",
            enc.display(),
            wua.display(),
        );
        let entries = parse_title_list(&xml).unwrap();
        let resolved = resolve_from_title_list(&dirs(dir.path()), &entries);
        assert!(resolved.encrypted);
        assert_eq!(resolved.wua, Some(wua.canonicalize().unwrap()));
        assert!(resolved.base.is_none());
    }

    #[test]
    fn relative_title_paths_resolve_in_dir_order() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        let d = dirs(&root);
        // only `local` has the relative dump, so resolution must reach it
        // after root and portable miss
        std::fs::create_dir_all(d.local.join("rel/base")).unwrap();
        let xml = "<cache><title \
                   titleId=\"00050000101C9400\"><format>1</format><path>rel/base</path></title></\
                   cache>";
        let entries = parse_title_list(xml).unwrap();
        let resolved = resolve_from_title_list(&d, &entries);
        assert_eq!(
            resolved.base,
            Some(d.local.join("rel/base").join("content"))
        );
    }

    #[test]
    fn resolves_from_mlc_dir_layout() {
        let dir = tempfile::tempdir().unwrap();
        let mlc = dir.path().join("mlc01");
        let base = mlc.join("usr/title/00050000/101C9400/content");
        let update = mlc.join("usr/title/0005000e/101C9500/content");
        let dlc = mlc.join("usr/title/0005000c/101C9300/content/0010");
        for p in [&base, &update, &dlc] {
            std::fs::create_dir_all(p).unwrap();
        }
        let resolved = resolve_from_mlc(&mlc, "<settings/>");
        assert_eq!(resolved.base, Some(base.clone()));
        assert_eq!(resolved.update, Some(update.clone()));
        assert_eq!(resolved.dlc, Some(dlc.clone()));
    }

    #[test]
    fn mlc_falls_back_to_scraped_rpx_path() {
        let dir = tempfile::tempdir().unwrap();
        let mlc = dir.path().join("mlc01");
        std::fs::create_dir_all(&mlc).unwrap();
        let xml = "<settings>\n<path>/games/BotW/code/U-King.rpx</path>\n</settings>";
        let resolved = resolve_from_mlc(&mlc, xml);
        assert_eq!(resolved.base, Some(PathBuf::from("/games/BotW/content")));
        assert!(resolved.update.is_none());
    }

    #[test]
    fn find_settings_xml_prefers_portable() {
        let dir = tempfile::tempdir().unwrap();
        let d = dirs(dir.path());
        let portable = d.portable.clone();
        std::fs::create_dir_all(&portable).unwrap();
        std::fs::write(portable.join("settings.xml"), "<settings/>").unwrap();
        assert_eq!(
            find_settings_xml(&d).unwrap(),
            portable.join("settings.xml")
        );
    }

    #[test]
    fn find_settings_xml_errors_when_missing() {
        let dir = tempfile::tempdir().unwrap();
        let d = dirs(dir.path());
        assert!(find_settings_xml(&d).is_err());
    }
}
