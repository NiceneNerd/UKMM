pub fn main() {
    let mut frame = sciter::Window::new();
    if cfg!(debug_assertions) {
        frame
            .set_options(sciter::window::Options::DebugMode(true))
            .unwrap();
    }
    let archived = include_bytes!("../../target/assets.rc");
    frame.archive_handler(archived).unwrap();
    frame.load_file("this://app/index.html");
    frame.run_app();
}
