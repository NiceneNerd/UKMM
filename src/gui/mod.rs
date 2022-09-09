use std::str::FromStr;

use sciter::{dispatch_script_call, Value, HELEMENT};

impl crate::mods::Mod {
    pub fn to_value(&self) -> Value {
        Value::from_str(&serde_json::to_string(self).unwrap()).unwrap()
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
                .map(|m| -> Value { m.to_value() })
                .collect()
        };

        let check_hash = |args: &[Value]| {
            println!("{}", args[0].to_int().unwrap() as u64);
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
