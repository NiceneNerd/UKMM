use std::{
    io::{stdin, stdout, Write},
    option::Option,
    path::{Path, PathBuf},
};

use anyhow_ext::{Context, Result};
use smartstring::alias::String;
use uk_manager::{core, mods::LookupMod, settings::Platform};
use uk_mod::{unpack::ModReader, Manifest, Meta};

use crate::gui::{package, tasks};

xflags::xflags! {
    src "./src/cli.rs"

    /// Command line interface for U-King Mod Manager
    cmd ukmm {
        /// Verbose logging for debugging
        optional -d, --debug
        /// Run using settings in same folder as executable
        optional -p, --portable
        /// Automatically deploy after running command (redunant with `deploy` command)
        optional -D, --deploy
        /// Install a mod
        cmd install {
            /// Path to the mod to install
            required path: PathBuf
            /// The profile to install the mod in
            optional profile: String
        }
        /// Package a mod
        cmd package {
            /// Path to the mod root directory
            required path: PathBuf
            /// Path to the output mod archive
            required output: PathBuf
            /// Path to the meta file for the mod
            required meta: PathBuf
        }
        /// Uninstall a mod
        cmd uninstall {
            /// The index of the mod to uninstall
            optional index: usize
            /// The profile to uninstall the mod from
            optional profile: String
        }
        /// Refresh merge
        cmd remerge {}
        /// Deploy mods
        cmd deploy {}
        /// Change current mode (Switch or Wii U)
        cmd mode {
            /// Mode to activate (Switch or Wii U)
            required platform: Platform
        }
    }
}
// generated start
// The following code is generated by `xflags` macro.
// Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
#[derive(Debug)]
pub struct Ukmm {
    pub debug: bool,
    /// While the `portable` flag needs to be recognized by xflags, it is
    /// handled by [std::env::args] in the static Settings methods
    #[allow(dead_code)]
    pub portable: bool,
    pub deploy: bool,
    pub subcommand: UkmmCmd,
}

#[derive(Debug)]
pub enum UkmmCmd {
    Install(Install),
    Uninstall(Uninstall),
    Package(Package),
    Remerge(Remerge),
    Deploy(Deploy),
    Mode(Mode),
}

#[derive(Debug)]
pub struct Install {
    pub path:    PathBuf,
    pub profile: Option<String>,
}

#[derive(Debug)]
pub struct Package {
    pub path:   PathBuf,
    pub output: PathBuf,
    pub meta:   PathBuf,
}

#[derive(Debug)]
pub struct Uninstall {
    pub index:   Option<usize>,
    pub profile: Option<String>,
}

#[derive(Debug)]
pub struct Remerge;

#[derive(Debug)]
pub struct Deploy;

#[derive(Debug)]
pub struct Mode {
    pub platform: Platform,
}

impl Ukmm {
    #[allow(dead_code)]
    pub fn from_env_or_exit() -> Self {
        Self::from_env_or_exit_()
    }

    #[allow(dead_code)]
    pub fn from_env() -> xflags::Result<Self> {
        Self::from_env_()
    }

    #[allow(dead_code)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
        Self::from_vec_(args)
    }
}
// generated end

macro_rules! input {
    () => {{
        stdout().flush()?;
        let mut _input = std::string::String::new();
        stdin().read_line(&mut _input).unwrap();
        _input
    }};
}

#[derive(Debug)]
pub struct Runner {
    core: core::Manager,
    cli:  Ukmm,
}

impl Runner {
    pub fn new(cli: Ukmm) -> Self {
        Self {
            core: core::Manager::init().unwrap(),
            cli,
        }
    }

    fn check_mod(&self, path: &Path) -> Result<Option<PathBuf>> {
        println!("Opening mod at {}...", path.display());
        let (mod_, path) = match ModReader::open(path, vec![]) {
            Ok(mod_) => (mod_, path.to_path_buf()),
            Err(e) => {
                match uk_manager::mods::convert_gfx(&self.core, path, None) {
                    Ok(path) => {
                        log::info!("Opening mod at {}", path.display());
                        (
                            ModReader::open(&path, vec![])
                                .context("Failed to open converted mod")?,
                            path,
                        )
                    }
                    Err(e2) => {
                        anyhow_ext::bail!(
                            "Could not open mod. Error when attempting to open as UKMM mod: {}. \
                             Error when attempting to open as legacy mod: {}.",
                            e,
                            e2
                        )
                    }
                }
            }
        };
        if !mod_.meta.options.is_empty() {
            anyhow_ext::bail!(
                "This mod contains configuration options and should be installed via the GUI."
            );
        }
        println!("Installing {}...", mod_.meta.name);
        Ok(Some(path))
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

    pub fn run(self) -> Result<()> {
        if self.cli.debug {
            env_logger::init();
            log::set_max_level(log::LevelFilter::Debug);
        }
        match &self.cli.subcommand {
            UkmmCmd::Mode(Mode { platform }) => {
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
            UkmmCmd::Install(Install { path, profile }) => {
                if let Some(path) = self.check_mod(path)? {
                    let mods = self.core.mod_manager();
                    let mod_ = mods.add(&path, profile.as_ref())?;
                    mods.set_enabled(mod_.as_map_id(), true, profile.as_ref())?;
                    mods.save()?;
                    println!("Applying mod to load order...");
                    let deployer = self.core.deploy_manager();
                    deployer.apply(Some(mod_.manifest()?.as_ref().clone()))?;
                    if self.cli.deploy {
                        self.deploy()?;
                    }
                    println!("Done!");
                }
            }
            UkmmCmd::Package(pkg) => {
                println!("Packaging mod...");
                let builder = package::ModPackerBuilder {
                    source: pkg.path.clone(),
                    dest:   pkg.output.clone(),
                    meta:   Meta::parse(&pkg.meta)?,
                };
                tasks::package_mod(&self.core, builder)?;
                println!("Done!");
            }
            UkmmCmd::Remerge(_) => {
                println!("Remerging...");
                tasks::apply_changes(&self.core, vec![], None)?;
                println!("Done!");
            }
            UkmmCmd::Uninstall(Uninstall { index, profile }) => {
                let mut manifests = Manifest::default();
                let mod_manager = self.core.mod_manager();
                let mods = mod_manager.mods().collect::<Vec<_>>();

                if let Some(index_value) = index {
                    let mod_ = mods
                        .get(*index_value)
                        .with_context(|| format!("Mod {} does not exist", index_value))?;
                    println!("Removing mod {}...", &mod_.meta.name);
                    mod_manager.del(mod_, profile.as_ref())?;
                    mod_manager.save()?;
                    manifests.extend(mod_.manifest()?.as_ref());
                } else {
                    println!("Installed mods:");
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
                        mod_manager.del(mod_, profile.as_ref())?;
                        mod_manager.save()?;
                        manifests.extend(mod_.manifest()?.as_ref());
                    }
                }

                println!("Applying changes to merge...");
                self.core.deploy_manager().apply(Some(manifests))?;
                if self.cli.deploy {
                    self.deploy()?;
                }
                println!("Done!");
            }
            UkmmCmd::Deploy(_) => self.deploy()?,
        };
        Ok(())
    }
}
