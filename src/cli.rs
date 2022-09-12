use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use fs_err as fs;
use std::{
    io::{stdin, stdout, Write},
    path::{Path, PathBuf},
};
use uk_mod::{pack::ModPacker, unpack::ModReader, Manifest};

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
pub struct Runner {
    core: crate::core::Manager,
    cli: Cli,
}

impl Runner {
    pub fn new(cli: Cli) -> Self {
        Self {
            core: crate::core::Manager::init().unwrap(),
            cli,
        }
    }

    fn check_mod(&self, path: &Path) -> Result<Option<PathBuf>> {
        println!("Opening mod at {}...", path.display());
        let (mod_, path) = match ModReader::open(path, vec![]) {
            Ok(mod_) => (mod_, path.to_path_buf()),
            Err(e) => {
                match crate::mods::convert_gfx(path) {
                    Ok(path) => {
                        log::info!("Opening mod at {}", path.display());
                        (ModReader::open(&path, vec![]).context("Failed to open converted mod")?, path)
                    },
                    Err(e2) => anyhow::bail!(
                        "Could not open mod. Error when attempting to open as UKMM mod: {}. Error when attempting to open as legacy mod: {}.",
                        e,
                        e2
                    )
                }
            }
        };
        if !mod_.meta.options.is_empty() {
            // anyhow::bail!(
            //     "This mod contains configuration options and should be installed via the GUI."
            // );
        }
        println!(
            "Identified mod: {} (v{}) by {}",
            &mod_.meta.name, &mod_.meta.version, &mod_.meta.author
        );
        print!("Do you want to continue installing? [Y/n] ");
        let cont = !input!().to_lowercase().starts_with('n');
        if cont {
            println!("Installing {}...", mod_.meta.name);
            Ok(Some(path))
        } else {
            Ok(None)
        }
    }

    fn deploy(&self) -> Result<()> {
        let deployer = self.core.deploy_manager();
        if deployer.pending() {
            println!("Deploying changes...");
            deployer.deploy()?;
            println!("Deployment complete");
        } else {
            println!("No changes pending deployment");
        };
        Ok(())
    }

    pub fn run(mut self) -> Result<()> {
        match self.cli.command.as_ref().unwrap() {
            Commands::Mode { platform } => {
                self.core
                    .settings_mut()
                    .apply(|s| s.current_mode = *platform)?;
                self.core.reload()?;
                println!("Mode changed to {:?}", platform);
                if self.cli.deploy {
                    self.deploy()?;
                }
                println!("Done!");
            }
            Commands::Install { path } => {
                if let Some(path) = self.check_mod(path)? {
                    let mods = self.core.mod_manager();
                    let mod_ = mods.add(&path)?;
                    mods.enable(mod_)?;
                    println!("Applying mod to load order...");
                    let deployer = self.core.deploy_manager();
                    deployer.apply(Some(mod_.manifest.clone()))?;
                    if self.cli.deploy {
                        self.deploy()?;
                    }
                    println!("Done!");
                }
            }
            Commands::Uninstall => {
                println!("Installed mods:");
                let mod_manager = self.core.mod_manager();
                let mods = mod_manager.mods().map(|m| m.clone()).collect::<Vec<_>>();
                for (i, mod_) in mods.iter().enumerate() {
                    println!(
                        "{}. {} (v{}) by {}",
                        i + 1,
                        &mod_.meta.name,
                        &mod_.meta.version,
                        &mod_.meta.author
                    );
                }
                print!("Enter mod(s) to uninstall, separated by commas: ");
                let mut manifests = Manifest::default();
                for id in input!().replace(' ', "").split(',') {
                    let mod_ = mods
                        .get(id.trim().parse::<usize>().context("Invalid mod number")? - 1)
                        .with_context(|| format!("Mod {} does not exist", id))?;
                    println!("Removing mod {}...", &mod_.meta.name);
                    mod_manager.del(mod_)?;
                    manifests.extend(&mod_.manifest);
                }
                println!("Applying changes to merge...");
                self.core.deploy_manager().apply(Some(manifests))?;
                if self.cli.deploy {
                    self.deploy()?;
                }
                println!("Done!");
            }
            Commands::Deploy => self.deploy()?,
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
            profile: "Default".into(),
        });
        std::fs::write(
            "/home/nn/.config/ukmm/settings.toml",
            toml::to_string(&settings).unwrap(),
        )
        .unwrap();
    }
}
