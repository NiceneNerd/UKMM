use anyhow::Result;
use clap::{Parser, Subcommand};
use std::{
    io::{stdin, stdout, Write},
    path::{Path, PathBuf},
};
use uk_mod::unpack::ModReader;

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Install a mod
    Install { path: PathBuf },
    /// Uninstall a mod
    Uninstall,
    /// Deploy mods
    Deploy,
    /// Change current mode (Switch or Wii U)
    Mode { platform: crate::settings::Platform },
}

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    #[clap(subcommand)]
    pub command: Option<Commands>,
    /// Automatically deploy after running command (redunant with `deploy` command)
    #[clap(short, long)]
    pub deploy: bool,
}

macro_rules! input {
    () => {{
        stdout().flush()?;
        let mut _input = String::new();
        stdin().read_line(&mut _input).unwrap();
        _input
    }};
}

#[derive(Debug)]
pub struct Runner<'a> {
    core: &'a mut crate::core::Manager,
    cli: Cli,
}

impl<'a> Runner<'a> {
    pub fn new(core: &'a mut crate::core::Manager, cli: Cli) -> Self {
        Self { core, cli }
    }

    fn check_mod(&self, path: &Path) -> Result<bool> {
        println!("Opening mod at {}...", path.display());
        let mod_ = ModReader::open(path, vec![])?;
        if !mod_.meta.options.is_empty() {
            // anyhow::bail!(
            //     "This mod contains configuration options and should be installed via the GUI."
            // );
        }
        println!(
            "Identified mod: {} (v. {}) by {}",
            &mod_.meta.name, &mod_.meta.version, &mod_.meta.author
        );
        print!("Do you want to continue installing? [Y/n] ");
        let cont = !input!().to_lowercase().starts_with('n');
        if cont {
            println!("Installing {}...", mod_.meta.name);
        }
        Ok(cont)
    }

    pub fn run(self) -> Result<()> {
        match self.cli.command.as_ref().unwrap() {
            Commands::Mode { platform } => {
                self.core
                    .settings_mut()
                    .apply(|s| s.current_mode = *platform)?;
                self.core.reload()?;
                if self.cli.deploy {
                    let deployer = self.core.deploy_manager();
                    if deployer.pending() {
                        println!("Deploying changes...");
                        deployer.deploy()?;
                    } else {
                        println!("No changes pending deployment");
                    }
                }
            }
            Commands::Install { path } => {
                if !self.check_mod(path)? {
                    return Ok(());
                }
                let mods = self.core.mod_manager();
                let mod_ = mods.add(path)?;
                mods.enable(mod_)?;
                println!("Applying mod to load order...");
                let deployer = self.core.deploy_manager();
                deployer.apply(Some(mod_.manifest.clone()))?;
                if self.cli.deploy {
                    println!("Deploying changes...");
                    deployer.deploy()?;
                }
            }
            Commands::Uninstall => todo!(),
            Commands::Deploy => {
                let deployer = self.core.deploy_manager();
                if deployer.pending() {
                    println!("Deploying changes...");
                    deployer.deploy()?;
                } else {
                    println!("No changes pending deployment");
                }
            }
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use uk_reader::ResourceReader;

    use crate::settings::{DeployConfig, Platform};

    #[test]
    fn settings() {
        let mut settings = crate::settings::Settings::default();
        settings.current_mode = Platform::WiiU;
        settings.wiiu_config = Some(crate::settings::PlatformSettings {
            language: crate::settings::Language::USen,
            dump: Arc::new(
                ResourceReader::from_unpacked_dirs(
                    Some("/games/Cemu/mlc01/usr/title/00050000/101C9400/content"),
                    Some("/games/Cemu/mlc01/usr/title/0005000E/101C9400/content"),
                    Some("/games/Cemu/mlc01/usr/title/0005000C/101C9400/content/0010"),
                )
                .unwrap(),
            ),
            deploy_config: Some(DeployConfig {
                auto: false,
                method: crate::settings::DeployMethod::HardLink,
                output: "/tmp/BreathOfTheWild_UKMM".into(),
            }),
        });
        std::fs::write(
            "/home/nn/.config/ukmm/settings.toml",
            toml::to_string(&settings).unwrap(),
        )
        .unwrap();
    }
}
