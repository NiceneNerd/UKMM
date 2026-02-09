#![allow(unused_imports)]

use std::{env, fs, path::PathBuf, process::Command};

use anyhow::{anyhow, Context, Result};
#[cfg(target_os = "windows")]
use winreg::enums::*;
#[cfg(target_os = "windows")]
use winreg::RegKey;

pub fn register_handlers() -> Result<()> {
    #[cfg(target_os = "windows")]
    win_create_handler()?;
    #[cfg(target_os = "linux")]
    linux_create_handler()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn linux_create_handler() -> Result<()> {
    let home_dir = dirs2::home_dir().context("Failed to find home directory")?;
    let schema_file = home_dir
        .join(".local")
        .join("share")
        .join("applications")
        .join("ukmm.desktop");

    if schema_file.exists() {
        return Ok(());
    }

    let desktop = format!(
        "[Desktop Entry]
Type=Application
Name=UKMM
Comment=Starts U-King Mod Manager
Exec={} %u
StartupNotify=false
MimeType=x-scheme-handler/bcml;",
        env::current_exe()?.display()
    );

    fs::create_dir_all(schema_file.parent().unwrap()).context("Failed to create directories")?;
    fs::write(&schema_file, desktop.trim()).context("Failed to write to schema file")?;
    Command::new("xdg-mime")
        .args([
            "default",
            schema_file.to_str().unwrap(),
            "x-scheme-handler/bcml",
        ])
        .status()
        .context("Failed to execute xdg-mime command")?;
    Command::new("update-desktop-database")
        .arg(schema_file.parent().unwrap())
        .status()
        .context("Failed to update desktop files database")?;
    Ok(())
}

#[cfg(target_os = "windows")]
fn win_create_handler() -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let bcml_key = hkcu
        .create_subkey("Software\\Classes\\bcml")
        .context("Failed to create or open bcml registry key")?;
    let exec_path = env::current_exe().unwrap().to_string_lossy().to_string();

    let command_key_path = "Software\\Classes\\bcml\\shell\\open\\command";
    match hkcu.open_subkey_with_flags(command_key_path, KEY_READ) {
        Ok(okey) => {
            let value: String = okey.get_value("").context("Failed to get registry value")?;
            if !value.contains(&exec_path) {
                set_windows_registry(&bcml_key.0, &exec_path)?;
            }
        }
        Err(_) => {
            set_windows_registry(&bcml_key.0, &exec_path)?;
        }
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn set_windows_registry(bcml_key: &RegKey, exec_path: &str) -> Result<()> {
    bcml_key
        .set_value("URL Protocol", &"")
        .context("Failed to set URL Protocol")?;
    let shell_open_key = bcml_key
        .create_subkey("shell\\open\\command")
        .context("Failed to create shell\\open\\command subkey")?;
    shell_open_key
        .0
        .set_value("", &format!(r#"{} "%1""#, exec_path))
        .context("Failed to set command value")?;
    Ok(())
}
