#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::any::Any;
use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
include!(concat!(env!("OUT_DIR"), "/build_info.rs"));
mod cli;
mod gui;
mod logger;

use anyhow_ext::Result;
use cli::Ukmm;

#[cfg(target_os = "windows")]
#[link(name = "Kernel32")]
unsafe extern "system" {
    fn AttachConsole(pid: i32) -> bool;
}
#[cfg(target_os = "windows")]
#[link(name = "User32")]
unsafe extern "system" {
    fn MessageBoxW(hwnd: i32, message: *const i8, title: *const i8, utype: usize) -> i32;
}

fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    unsafe {
        AttachConsole(-1);
    }

    // I don't know why I need this and I hate it.
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .unwrap();

    match Ukmm::from_env() {
        Ok(command) => {
            cli::Runner::new(command).run()?;
        }
        Err(e) => {
            if !e.is_help() {
                let ipc = uk_app::host::oneclick::start_listener();
                if let Some(arg) = std::env::args().nth(1) {
                    if std::env::args().any(|a| a == "--debug") {
                        env_logger::init();
                        log::set_max_level(log::LevelFilter::Debug);
                    }
                    match uk_app::host::args::classify_arg(Some(arg)) {
                        uk_app::host::args::ArgAction::OneClick(url) => {
                            uk_app::host::oneclick::oneclick(&url);
                        }
                        uk_app::host::args::ArgAction::ModPath(path) => {
                            uk_app::host::oneclick::open_mod_arg(path);
                        }
                        uk_app::host::args::ArgAction::None => {}
                    }
                }
                std::panic::set_hook(Box::new(|_| {
                    println!("{}", std::backtrace::Backtrace::force_capture());
                }));
                if let Err(e) =
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| gui::main(ipc)))
                {
                    display_error(e);
                    if let Some(file) = logger::LOGGER.log_path() {
                        logger::LOGGER.save_log();
                        println!(
                            "More information may be available in the log file at {}. You can run \
                             with the --debug flag for additional detail.",
                            file.display()
                        );
                    }
                }
            } else {
                Ukmm::from_env_or_exit();
            }
        }
    }
    Ok(())
}

fn display_error(e: Box<dyn Any + Send>) {
    let error_msg = format!(
        "An unrecoverable error occurred. Error details: {}",
        e.downcast::<String>()
            .or_else(|e| {
                e.downcast::<&'static str>().map(|s| Box::new((*s).into()))
            })
            .unwrap_or_else(|_| {
                Box::new(
                    "An unknown error occurred, check the log for possible details."
                        .to_string(),
                )
            })
    );
    #[cfg(windows)]
    unsafe {
        let error_msg = error_msg.encode_utf16().collect::<Vec<u16>>();
        let title = "Error".encode_utf16().collect::<Vec<u16>>();
        MessageBoxW(
            0,
            error_msg.as_ptr() as *const i8,
            title.as_ptr() as *const i8,
            0x0 | 0x10,
        );
    }
    #[cfg(not(windows))]
    println!("{}", error_msg);
}
