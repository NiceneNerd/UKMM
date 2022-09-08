fn main() {
    std::process::Command::new("packfolder")
        .args(&["assets", "target/assets.rc", "-binary"])
        .output()
        .expect("Unable to run packfolder!");
}
