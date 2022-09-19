use super::EventHandler;
use crate::mods::Mod;
use rustc_hash::FxHashMap;
use sciter::{make_args, Value};
use std::{ops::Deref, panic::UnwindSafe, path::PathBuf};
use uk_mod::{unpack::ModReader, Manifest};

#[allow(non_snake_case)]
impl EventHandler {
    pub fn res<T>(
        &self,
        task: impl Send + Sync + FnOnce() -> anyhow::Result<T> + UnwindSafe + 'static,
        callback: Value,
    ) where
        T: Into<Value>,
    {
        std::thread::spawn(move || {
            let res = match std::panic::catch_unwind(task) {
                Ok(Ok(res)) => res.into(),
                e => {
                    let err: anyhow::Error = match e {
                        Ok(Err(e)) => e,
                        Err(e) => anyhow::Error::msg(
                            e.downcast_ref::<String>()
                                .cloned()
                                .or_else(|| e.downcast_ref::<&'static str>().map(|s| s.to_string()))
                                .unwrap(),
                        ),
                        _ => unreachable!(),
                    };
                    log::error!("{:?}", &err);
                    let mut res = Value::error(&err.to_string());
                    res.set_item("msg", Value::from(err.to_string()));
                    res.set_item("backtrace", Value::from(format!("{:?}", err)));
                    res
                }
            };
            callback.call(None, &make_args!(res), None).unwrap();
        });
    }

    pub fn mods(&self) -> Value {
        let mods = self
            .core
            .mod_manager()
            .all_mods()
            .map(|m| <&Mod as Into<Value>>::into(&m))
            .collect();
        log::debug!("Mods: {:?}", &mods);
        mods
    }

    pub fn profiles(&self) -> Value {
        let profiles = self
            .core
            .settings()
            .profiles()
            .map(|p| Value::from(p.as_str()))
            .collect();
        log::debug!("Profiles: {:?}", &profiles);
        profiles
    }

    pub fn currentProfile(&self) -> Value {
        self.core
            .settings()
            .platform_config()
            .map(|config| Value::from(config.profile.as_str()))
            .unwrap_or_else(|| Value::from("Default"))
    }

    pub fn preview(&self, hash: String) -> Value {
        let mod_manager = self.core.mod_manager();
        let hash = hash.parse::<usize>().unwrap();
        let mod_ = mod_manager.get_mod(hash).unwrap();
        if let Ok(Some(data)) = mod_.preview() {
            match &data[..4] {
                [0xff, 0xd8, 0xff, 0xe0] => {
                    Value::from(format!("data:image/jpeg;base64,{}", base64::encode(&data)))
                }
                [0x89, 0x50, 0x4e, 0x47] => {
                    Value::from(format!("data:image/png;base64,{}", base64::encode(&data)))
                }
                _ => {
                    log::debug!("Unsupported preview image, ignoring");
                    Value::null()
                }
            }
        } else {
            Value::null()
        }
    }

    pub fn parse_mod(&self, path: String, callback: Value) {
        let path: PathBuf = path.into();
        self.res(
            move || {
                log::info!("Opening mod at {}", path.display());
                ModReader::open(&path, vec![]).map(Mod::from_reader)
            },
            callback,
        );
    }

    pub fn settings(&self) -> Value {
        sciter_serde::to_value(self.core.settings().deref()).unwrap()
    }

    pub fn apply(&self, mods: String, callback: Value) {
        let changed_mods: Vec<Mod> = serde_json::from_str(&mods).unwrap();
        let core = self.core.clone();
        self.res(
            move || {
                log::info!("Parsing changes to mod configuration");
                let manager = core.mod_manager();
                let mods = manager.all_mods().map(|m| m.clone()).collect::<Vec<_>>();
                let order = changed_mods.iter().map(|m| m.hash).collect::<Vec<_>>();
                let mut manifest = Manifest::default();
                let mut modified = FxHashMap::default();
                for (i, mod_) in changed_mods.into_iter().enumerate() {
                    if modified.contains_key(&mod_.hash) {
                        modified.remove(&mod_.hash);
                        continue;
                    }
                    let original = &mods[i - modified.len()];
                    if &mod_ != original || !mod_.state_eq(original) {
                        log::debug!("The state of {} has been modified", original.meta.name);
                        modified.insert(mod_.hash, mod_.clone());
                        if mod_.enabled_options != original.enabled_options {
                            manifest.extend(
                                &manager.set_enabled_options(mod_.hash, mod_.enabled_options)?,
                            );
                            continue;
                        }
                        if mod_.enabled != original.enabled {
                            manager.set_enabled(mod_.hash, mod_.enabled)?;
                        }
                        manifest.extend(&mod_.manifest);
                    }
                }
                if order.iter().ne(mods.iter().map(|m| &m.hash)) {
                    log::info!("Updating load order");
                    manager.set_order(order);
                }
                manager.save()?;
                core.deploy_manager().apply(Some(manifest))?;
                log::info!("Completed applying changes");
                Ok(())
            },
            callback,
        );
    }
}
