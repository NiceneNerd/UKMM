#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::any::Any;

use mimalloc::MiMalloc;

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;
include!(concat!(env!("OUT_DIR"), "/build_info.rs"));
mod gui;
mod logger;

use anyhow_ext::Result;
use uk_cli::Ukmm;

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
            uk_cli::run(command)?;
        }
        Err(e) => {
            if !e.is_help() {
                // A malformed CLI invocation must not fall through to the GUI
                // — on a headless machine that would exit 0 while doing
                // nothing. That is any argument naming a subcommand that failed
                // to parse, or a first argument that is neither a flag, a
                // `bcml:` URL, nor an existing path (i.e. a typo'd command).
                let args: Vec<std::ffi::OsString> = std::env::args_os().collect();
                let cli_error = args
                    .iter()
                    .skip(1)
                    .any(|a| uk_cli::SUBCOMMANDS.contains(&a.to_string_lossy().as_ref()))
                    || match args.get(1) {
                        Some(a) => {
                            let s = a.to_string_lossy();
                            !s.starts_with('-')
                                && !s.starts_with("bcml:")
                                && !std::path::Path::new(s.as_ref()).exists()
                        }
                        None => false,
                    };
                if cli_error {
                    eprintln!("Error: {e}");
                    eprintln!("Run `ukmm --help` for usage.");
                    std::process::exit(2);
                }
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
            .or_else(|e| { e.downcast::<&'static str>().map(|s| Box::new((*s).into())) })
            .unwrap_or_else(|_| {
                Box::new(
                    "An unknown error occurred, check the log for possible details.".to_string(),
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
