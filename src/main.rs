#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]
mod cli;
mod gui;
mod logger;

use anyhow_ext::Result;
use cli::Ukmm;

#[cfg(target_os = "windows")]
#[link(name = "Kernel32")]
extern "system" {
    fn AttachConsole(pid: i32) -> bool;
}
#[cfg(target_os = "windows")]
#[link(name = "User32")]
extern "system" {
    fn MessageBoxW(hwnd: i32, message: *const i8, title: *const i8, utype: usize) -> i32;
}

fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    unsafe {
        AttachConsole(-1);
    }

    let gui_flags = ["-p", "--portable", "-d", "--debug"];
    if std::env::args().count() == 1
        || std::env::args()
            .skip(1)
            .all(|a| gui_flags.contains(&a.as_str()))
    {
        if let Err(e) = std::panic::catch_unwind(gui::main) {
            let error_msg = format!(
                "An unrecoverable error occured. Error details: {}",
                e.downcast::<String>()
                    .or_else(|e| e.downcast::<&'static str>().map(|s| Box::new((*s).into())))
                    .unwrap_or_else(|_| {
                        Box::new(
                            "An unknown error occured, check the log for possible details."
                                .to_string(),
                        )
                    })
            );
            #[cfg(windows)]
            MessageBoxW(
                0,
                core::ffi::CStr::from(&error_msg).as_ptr(),
                core::ffi::CStr::from("Error").as_ptr(),
                0x0 | 0x10,
            );
            #[cfg(not(windows))]
            println!("{}", error_msg);
            if let Some(file) = logger::LOGGER.log_path() {
                logger::LOGGER.save_log();
                println!(
                    "More information may be available in the log file at {}. You can run with \
                     the --debug flag for additional detail.",
                    file.display()
                );
            }
        }
    } else {
        let cmd = Ukmm::from_env_or_exit();
        cli::Runner::new(cmd).run()?;
    }
    Ok(())
}
