//! Test fixtures shared by uk-app's own tests, its integration tests, and
//! frontend crates (e.g. the CLI smoke test in the root package). Not public
//! API — anything here may change at any time.

use std::{
    io::Write,
    path::{Path, PathBuf},
};

use uk_mod::{Manifest, Meta, ModPlatform};

/// Creates a minimal fake unpacked dump at `root/content` (the reader checks
/// that this file exists) and returns the content dir.
pub fn fake_dump(root: &Path) -> PathBuf {
    let content = root.join("content");
    std::fs::create_dir_all(content.join("Map/MainField/A-1"))
        .expect("failed to create fake dump dir");
    std::fs::write(
        content.join("Map/MainField/A-1/A-1.00_Clustering.sblwp"),
        b"",
    )
    .expect("failed to write fake dump file");
    content
}

/// A UKMM mod zip with the given file name and mod name.
pub fn mod_zip_named(dir: &Path, file_name: &str, mod_name: &str) -> PathBuf {
    let mut meta = Meta::blank(ModPlatform::Universal);
    meta.name = mod_name.into();
    let path = dir.join(file_name);
    let mut zip =
        zip::ZipWriter::new(std::fs::File::create(&path).expect("failed to create mod zip"));
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
