//! Bulk conversion test: walks a BotW romfs dump and attempts to convert
//! every BFRES file from Wii U to Switch format.
//!
//! Run against the default romfs paths:
//!   cargo test -p uk-bfres --test bulk_conversion -- --ignored --nocapture
//!
//! Or specify a custom romfs path via environment variable:
//!   BFRES_TEST_ROMFS=/path/to/wiiu/romfs cargo test -p uk-bfres --test bulk_conversion -- --ignored --nocapture
//!
//! The test also converts all Wii U fixture files in tests/fixtures/ as a
//! sanity check (this part always runs, even without a romfs dump).

use std::collections::BTreeMap;
use std::fs;
use std::panic;
use std::path::{Path, PathBuf};

/// Default base game romfs path (Switch dump -- title ID 01007EF00011E000)
const DEFAULT_BASE_ROMFS: &str = concat!(
    env!("HOME"),
    "/fun/nintendo hacking/mods/botw romfs/contents/01007EF00011E000/romfs"
);

/// Default DLC romfs path
const DEFAULT_DLC_ROMFS: &str = concat!(
    env!("HOME"),
    "/fun/nintendo hacking/mods/botw romfs/contents/01007EF00011F001/romfs"
);

/// Yaz0 magic bytes
const YAZ0_MAGIC: &[u8; 4] = b"Yaz0";

struct ConversionResults {
    total_files: usize,
    wiiu_bfres_count: usize,
    switch_bfres_count: usize,
    successfully_converted: usize,
    failed: Vec<(PathBuf, String)>,
    panicked: Vec<(PathBuf, String)>,
    yaz0_decompressed: usize,
    yaz0_decompress_failed: Vec<(PathBuf, String)>,
    not_bfres: usize,
    too_small: usize,
}

impl ConversionResults {
    fn new() -> Self {
        Self {
            total_files: 0,
            wiiu_bfres_count: 0,
            switch_bfres_count: 0,
            successfully_converted: 0,
            failed: Vec::new(),
            panicked: Vec::new(),
            yaz0_decompressed: 0,
            yaz0_decompress_failed: Vec::new(),
            not_bfres: 0,
            too_small: 0,
        }
    }

    fn merge(&mut self, other: ConversionResults) {
        self.total_files += other.total_files;
        self.wiiu_bfres_count += other.wiiu_bfres_count;
        self.switch_bfres_count += other.switch_bfres_count;
        self.successfully_converted += other.successfully_converted;
        self.failed.extend(other.failed);
        self.panicked.extend(other.panicked);
        self.yaz0_decompressed += other.yaz0_decompressed;
        self.yaz0_decompress_failed.extend(other.yaz0_decompress_failed);
        self.not_bfres += other.not_bfres;
        self.too_small += other.too_small;
    }

    fn print_summary(&self) {
        println!();
        println!("========================================");
        println!("Bulk conversion results:");
        println!("  Total files scanned:              {}", self.total_files);
        println!("  Yaz0 decompressed:                {}", self.yaz0_decompressed);
        println!(
            "  Yaz0 decompress failed:           {}",
            self.yaz0_decompress_failed.len()
        );
        println!("  Too small to identify:            {}", self.too_small);
        println!("  Not BFRES / unrecognized:         {}", self.not_bfres);
        println!(
            "  Switch BFRES (no conversion):     {}",
            self.switch_bfres_count
        );
        println!("  Wii U BFRES files:                {}", self.wiiu_bfres_count);
        println!(
            "  Successfully converted:           {}",
            self.successfully_converted
        );
        println!("  Failed (graceful error):          {}", self.failed.len());
        println!("  Panicked:                         {}", self.panicked.len());
        println!("========================================");

        if self.wiiu_bfres_count == 0 && self.switch_bfres_count > 0 {
            println!();
            println!("  NOTE: All BFRES files are Switch format -- this is a Switch romfs dump.");
            println!("  To test Wii U -> Switch conversion, provide a Wii U BotW romfs dump.");
            println!("  Set BFRES_TEST_ROMFS=/path/to/wiiu/romfs and re-run.");
        }

        if !self.yaz0_decompress_failed.is_empty() {
            print_grouped(
                "Yaz0 decompression failures",
                &self.yaz0_decompress_failed,
                5,
            );
        }

        if !self.failed.is_empty() {
            print_grouped("Conversion failures (graceful errors)", &self.failed, 5);
        }

        if !self.panicked.is_empty() {
            print_grouped("PANICS", &self.panicked, 5);
        }
    }
}

/// Simple recursive directory walker.
fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_dir_inner(dir, &mut results);
    results
}

fn walk_dir_inner(dir: &Path, results: &mut Vec<PathBuf>) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        if path.is_dir() {
            walk_dir_inner(&path, results);
        } else {
            results.push(path);
        }
    }
}

/// Find all files with a given extension in a directory tree.
fn find_files_with_extension(dir: &Path, ext: &str) -> Vec<PathBuf> {
    walk_dir(dir)
        .into_iter()
        .filter(|p| {
            p.extension()
                .map(|e| e.eq_ignore_ascii_case(ext))
                .unwrap_or(false)
        })
        .collect()
}

/// Attempt to classify and convert a BFRES buffer, recording results.
fn try_convert(data: &[u8], path: &Path, results: &mut ConversionResults) {
    if data.len() < 12 {
        results.too_small += 1;
        return;
    }

    if uk_bfres::is_wiiu_bfres(data) {
        results.wiiu_bfres_count += 1;

        let data_owned = data.to_vec();
        let convert_result =
            panic::catch_unwind(|| uk_bfres::convert_wiiu_to_switch(&data_owned));

        match convert_result {
            Ok(Ok(_)) => {
                results.successfully_converted += 1;
            }
            Ok(Err(e)) => {
                results.failed.push((path.to_path_buf(), format!("{e}")));
            }
            Err(panic_info) => {
                let msg = if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "unknown panic".to_string()
                };
                results.panicked.push((path.to_path_buf(), msg));
            }
        }
    } else if uk_bfres::is_switch_bfres(data) {
        results.switch_bfres_count += 1;
    } else {
        results.not_bfres += 1;
    }
}

/// Read a file, decompress if yaz0, then classify and convert.
fn process_file(path: &Path, results: &mut ConversionResults) {
    results.total_files += 1;

    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => {
            results
                .failed
                .push((path.to_path_buf(), format!("IO error: {e}")));
            return;
        }
    };

    if data.len() >= 4 && &data[0..4] == YAZ0_MAGIC {
        match roead::yaz0::decompress(&data) {
            Ok(decompressed) => {
                results.yaz0_decompressed += 1;
                try_convert(&decompressed, path, results);
            }
            Err(e) => {
                results
                    .yaz0_decompress_failed
                    .push((path.to_path_buf(), format!("{e}")));
            }
        }
    } else {
        try_convert(&data, path, results);
    }
}

/// Process a romfs directory, finding and converting all BFRES / SBFRES files.
fn process_romfs(romfs_path: &Path, label: &str) -> ConversionResults {
    let mut results = ConversionResults::new();

    if !romfs_path.exists() {
        println!("  [{label}] Path does not exist, skipping: {}", romfs_path.display());
        return results;
    }

    println!("  [{label}] Scanning: {}", romfs_path.display());

    let bfres_files = find_files_with_extension(romfs_path, "bfres");
    let sbfres_files = find_files_with_extension(romfs_path, "sbfres");

    println!(
        "  [{label}] Found {} .bfres files, {} .sbfres files",
        bfres_files.len(),
        sbfres_files.len()
    );

    for path in &bfres_files {
        process_file(path, &mut results);
    }

    let total_sbfres = sbfres_files.len();
    for (i, path) in sbfres_files.iter().enumerate() {
        if total_sbfres > 100 && (i + 1) % 500 == 0 {
            println!("  [{label}] Processing sbfres {}/{}...", i + 1, total_sbfres);
        }
        process_file(path, &mut results);
    }

    results
}

/// Print a grouped summary of path+error pairs.
fn print_grouped(header: &str, entries: &[(PathBuf, String)], max_examples: usize) {
    println!();
    println!("--- {header} ---");
    let mut groups: BTreeMap<String, Vec<&PathBuf>> = BTreeMap::new();
    for (path, msg) in entries {
        groups.entry(msg.clone()).or_default().push(path);
    }
    for (msg, paths) in &groups {
        println!();
        println!("  Error: {msg}");
        println!("  Count: {}", paths.len());
        for (i, path) in paths.iter().enumerate() {
            if i >= max_examples {
                println!("    ... and {} more", paths.len() - max_examples);
                break;
            }
            println!("    - {}", path.display());
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Bulk conversion against the romfs dump. Requires the dump to be present.
/// Marked #[ignore] so it doesn't run by default.
#[test]
#[ignore]
fn bulk_convert_botw_bfres() {
    // Check for custom romfs path via env var
    let custom_romfs = std::env::var("BFRES_TEST_ROMFS").ok();

    let romfs_paths: Vec<(&str, String)> = if let Some(ref custom) = custom_romfs {
        vec![("Custom", custom.clone())]
    } else {
        vec![
            ("Base", DEFAULT_BASE_ROMFS.to_string()),
            ("DLC", DEFAULT_DLC_ROMFS.to_string()),
        ]
    };

    // Check if any path exists
    let any_exists = romfs_paths.iter().any(|(_, p)| Path::new(p).exists());
    if !any_exists {
        println!("=== SKIPPING bulk conversion test ===");
        println!("No romfs paths exist:");
        for (label, path) in &romfs_paths {
            println!("  {label}: {path}");
        }
        println!();
        println!("To run this test, provide a BotW romfs dump at the paths above,");
        println!("or set BFRES_TEST_ROMFS=/path/to/romfs");
        return;
    }

    println!();
    println!("=== Bulk BFRES Conversion Test ===");
    println!();

    let mut total = ConversionResults::new();

    for (label, path) in &romfs_paths {
        let results = process_romfs(Path::new(path), label);
        total.merge(results);
    }

    total.print_summary();

    // The test fails only if there were panics
    assert!(
        total.panicked.is_empty(),
        "BFRES conversion panicked on {} files! See output above for details.",
        total.panicked.len()
    );

    println!();
    println!("Test PASSED: No panics encountered.");
    if !total.failed.is_empty() {
        println!(
            "Note: {} files had graceful errors (see above).",
            total.failed.len()
        );
    }
}

/// Convert all Wii U fixture files as a sanity check.
/// This always runs (not ignored) since fixtures are checked into the repo.
#[test]
fn convert_fixture_files() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    if !fixtures_dir.exists() {
        println!("No fixtures directory found, skipping");
        return;
    }

    let mut results = ConversionResults::new();

    // Find all .wiiu.bfres files in fixtures
    let all_files = walk_dir(&fixtures_dir);
    let wiiu_fixtures: Vec<_> = all_files
        .into_iter()
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().contains(".wiiu.bfres"))
                .unwrap_or(false)
        })
        .collect();

    println!();
    println!("=== Fixture Conversion Test ===");
    println!("  Found {} Wii U fixture files", wiiu_fixtures.len());

    for path in &wiiu_fixtures {
        process_file(path, &mut results);
    }

    results.print_summary();

    assert!(
        results.panicked.is_empty(),
        "Fixture conversion panicked on {} files!",
        results.panicked.len()
    );

    // For fixtures, we also want zero failures since these are known-good files
    assert!(
        results.failed.is_empty(),
        "Fixture conversion failed on {} files! See output above.",
        results.failed.len()
    );

    assert!(
        results.wiiu_bfres_count > 0,
        "Expected to find Wii U BFRES fixture files but found none"
    );

    assert_eq!(
        results.successfully_converted, results.wiiu_bfres_count,
        "Not all Wii U fixtures converted successfully: {}/{}",
        results.successfully_converted, results.wiiu_bfres_count
    );

    println!();
    println!(
        "Test PASSED: All {} fixture files converted successfully.",
        results.successfully_converted
    );
}
