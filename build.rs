#[cfg(windows)]
extern crate winres;

fn copyright_string() {
    use std::io::Write;

    use astrolabe::DateUtilities;
    let start = 2022;
    let year = astrolabe::Date::now().year();
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let dest_path = format!("{}/build_info.rs", out_dir);
    let mut file = std::fs::File::create(dest_path).unwrap();

    writeln!(
        &mut file,
        "pub const COPYRIGHT: &str = \"© {}-{} Caleb Smith, Ginger Chody - GPLv3\";",
        start,
        year
    )
    .unwrap();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/main.rs");
}

#[cfg(windows)]
fn add_icon() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/ukmm.ico");
    res.compile().unwrap();
}

fn main() {
    #[cfg(windows)]
    add_icon();
    copyright_string();
}
