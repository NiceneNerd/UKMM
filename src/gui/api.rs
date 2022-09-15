use std::ops::Deref;

use crate::core::Manager;
use sciter::Value;

impl Manager {
    pub fn api(&self) -> Value {
        let mods = |_args: &[Value]| -> Value {
            let mods = self
                .mod_manager()
                .all_mods()
                .map(|m| m.to_value())
                .collect();
            log::debug!("Mods: {:?}", &mods);
            mods
        };

        let profiles = |_args: &[Value]| -> Value {
            let profiles = self
                .settings()
                .profiles()
                .map(|p| Value::from(p.as_str()))
                .collect();
            log::debug!("Profiles: {:?}", &profiles);
            profiles
        };

        let current_profile = |_args: &[Value]| -> Value {
            self.settings()
                .platform_config()
                .map(|config| Value::from(config.profile.as_str()))
                .unwrap_or_else(|| Value::from("Default"))
        };

        let preview = |args: &[Value]| -> Value {
            let mod_manager = self.mod_manager();
            let hash = args[0].as_string().unwrap().parse::<usize>().unwrap();
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
        };

        let settings =
            |_args: &[Value]| -> Value { sciter_serde::to_value(self.settings().deref()).unwrap() };

        let check_hash = |args: &[Value]| {
            println!("{}", args[0].to_float().unwrap());
        };

        let mut api = Value::new();
        api.set_item("mods", mods);
        api.set_item("profiles", profiles);
        api.set_item("preview", preview);
        api.set_item("current_profile", current_profile);
        api.set_item("check_hash", check_hash);
        api.set_item("settings", settings);
        api
    }
}
