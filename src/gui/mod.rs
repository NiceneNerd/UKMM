use sciter::{dispatch_script_call, Value, HELEMENT};
use std::str::FromStr;

use crate::mods::LookupMod;

impl crate::mods::Mod {
    pub fn to_value(&self) -> Value {
        let mut val = Value::new();
        val.set_item("meta", sciter_serde::to_value(&self.meta).unwrap());
        val.set_item("manifest", sciter_serde::to_value(&self.manifest).unwrap());
        val.set_item(
            "enabled_options",
            sciter_serde::to_value(&self.enabled_options).unwrap(),
        );
        val.set_item("enabled", Value::from(self.enabled));
        val.set_item("path", Value::from(self.path.to_str().unwrap()));
        val.set_item("hash", Value::from(self.as_hash_id() as f64));
        val
    }
}

struct EventHandler {
    core: crate::core::Manager,
}

impl EventHandler {
    #[allow(non_snake_case)]
    fn GetApi(&mut self) -> Value {
        let mods = |_args: &[Value]| -> Value {
            self.core
                .mod_manager()
                .all_mods()
                .map(|m| m.to_value())
                .collect()
        };

        let check_hash = |args: &[Value]| {
            println!("{}", args[0].to_float().unwrap());
        };

        let mut api = Value::new();
        api.set_item("mods", mods);
        api.set_item("check_hash", check_hash);
        api
    }
}

impl sciter::EventHandler for EventHandler {
    dispatch_script_call! {
        fn GetApi();
    }

    fn on_script_call(&mut self, root: HELEMENT, name: &str, argv: &[Value]) -> Option<Value> {
        let handled = self.dispatch_script_call(root, name, argv);
        if handled.is_some() {
            return handled;
        }
        None
    }
}

pub fn main() {
    let mut frame = sciter::Window::new();
    frame.event_handler(EventHandler {
        core: crate::core::Manager::init().unwrap(),
    });
    if cfg!(debug_assertions) {
        frame
            .set_options(sciter::window::Options::DebugMode(true))
            .unwrap();
    }
    let archived = include_bytes!("../../target/assets.rc");
    frame.archive_handler(archived).unwrap();
    frame.load_file("this://app/index.html");
    frame.run_app();
}
