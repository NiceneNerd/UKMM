use crate::Commands;
use anyhow::Result;
use std::io::{stdin, stdout};
use uk_mod::unpack::ModReader;

macro_rules! input {
    () => {{
        let mut _input = String::new();
        stdin().read_line(&mut _input).unwrap();
        _input
    }};
}

pub fn process_command(core: &crate::core::Manager, command: Commands) -> Result<()> {
    match command {
        Commands::Mode { platform } => {
            core.settings_mut().apply(|s| s.current_mode = platform)?;
        }
        Commands::Install { path } => {
            println!("Opening mod at {}", path.display());
            let mod_ = ModReader::open(path, vec![])?;
            if !mod_.meta.options.is_empty() {
                println!(
                    "This mod contains configuration options and should be installed via the GUI."
                );
                return Ok(());
            }
            println!(
                "Identified mod: {} (v. {}) by {}",
                &mod_.meta.name, &mod_.meta.version, &mod_.meta.author
            );
            println!("Do you want to continue installing? [Y/n]");
            if input!().to_lowercase().starts_with('n') {
                return Ok(());
            }
            println!("Installing {}", &mod_.meta.name);
        }
        Commands::Uninstall => todo!(),
        Commands::Deploy => todo!(),
    };
    Ok(())
}
