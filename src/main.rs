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
extern "system" {
    fn AttachConsole(pid: i32) -> bool;
}
#[cfg(target_os = "windows")]
#[link(name = "User32")]
extern "system" {
    fn MessageBoxW(hwnd: i32, message: *const i8, title: *const i8, utype: usize) -> i32;
}

const INTERFACE: ssilide::Interface = ssilide::Interface::new(6666);

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
                if let Some(path) = std::env::args().nth(1) {
                    if std::env::args().any(|a| a == "--debug") {
                        env_logger::init();
                        log::set_max_level(log::LevelFilter::Debug);
                    }
                    match path.strip_prefix("bcml:") {
                        Some(url) => {
                            gui::tasks::oneclick(url);
                        }
                        None => {
                            gui::tasks::handle_mod_arg(path.into());
                        }
                    }
                }
                gui::tasks::wait_ipc();
                if let Err(e) = std::panic::catch_unwind(gui::main) {
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
