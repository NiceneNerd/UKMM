mod api;
use crate::mods::LookupMod;
use sciter::{dispatch_script_call, Element, Value, HELEMENT};
use std::sync::Arc;

impl From<&crate::mods::Mod> for Value {
    fn from(mod_: &crate::mods::Mod) -> Self {
        let mut val = Self::new();
        val.set_item("meta", sciter_serde::to_value(&mod_.meta).unwrap());
        val.set_item("manifest", sciter_serde::to_value(&mod_.manifest).unwrap());
        val.set_item(
            "enabled_options",
            sciter_serde::to_value(&mod_.enabled_options).unwrap(),
        );
        val.set_item("enabled", Value::from(mod_.enabled));
        val.set_item("path", Value::from(mod_.path.to_str().unwrap()));
        val.set_item("hash", Value::from(mod_.as_hash_id().to_string()));
        val
    }
}

impl From<crate::mods::Mod> for Value {
    fn from(mod_: crate::mods::Mod) -> Self {
        (&mod_).into()
    }
}

struct EventHandler {
    core: Arc<crate::core::Manager>,
    root: Option<Arc<Element>>,
}

impl sciter::EventHandler for EventHandler {
    dispatch_script_call! {
        fn mods();
        fn profiles();
        fn currentProfile();
        fn settings();
        fn preview(String);
        fn apply(String, Value);
        fn parse_mod(String, Value);
    }

    fn document_complete(&mut self, root: HELEMENT, _target: HELEMENT) {
        if self.root.is_none() {
            let mut root: Element = root.into();
            root.set_attribute(
                "theme",
                if self.core.settings().ui_config.dark {
                    "dark"
                } else {
                    "light"
                },
            )
            .unwrap_or_default();
            let root = Arc::new(root);
            crate::logger::LOGGER.set_root(Arc::clone(&root));
            self.root = Some(root);
            log::info!("Logger UI connected");
        }
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
    crate::logger::init();
    log::debug!("Logger initialized");
    let mut frame = sciter::Window::new();
    frame.event_handler(EventHandler {
        core: Arc::new(crate::core::Manager::init().unwrap()),
        root: None,
    });
    if cfg!(debug_assertions) {
        frame
            .set_options(sciter::window::Options::DebugMode(true))
            .unwrap();
    }
    let archived = include_bytes!("../../target/assets.rc");
    frame.archive_handler(archived).unwrap();
    frame.load_file("this://app/index.html");
    log::info!("Started ukmm");
    frame.run_app();
}
