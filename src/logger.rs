use chrono::NaiveDateTime;
use log::{LevelFilter, Record};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::Serialize;
use std::{ops::Deref, sync::Arc};

pub static LOGGER: Lazy<Logger> = Lazy::new(|| Logger {
    inner: env_logger::builder().build(),
    debug: std::env::args().any(|arg| &arg == "--debug"),
    queue: Mutex::new(vec![]),
    root: OnceCell::new(),
});

pub fn init() {
    log::set_logger(LOGGER.deref()).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}

#[derive(Debug, Serialize)]
pub struct Entry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub args: String,
}

impl From<&Record<'_>> for Entry {
    fn from(record: &Record) -> Self {
        Self {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            level: record.level().to_string(),
            target: record.target().to_string(),
            args: record.args().to_string(),
        }
    }
}

pub struct Logger {
    inner: env_logger::Logger,
    debug: bool,
    queue: Mutex<Vec<Entry>>,
    root: OnceCell<Arc<sciter::Element>>,
}

impl Logger {
    pub fn set_root(&self, root: Arc<sciter::Element>) {
        self.root.set(root).unwrap_or(());
    }

    pub fn flush_queue(&self) {
        if let Some(root) = self.root.get() {
            for entry in self.queue.lock().drain(..) {
                root.call_function(
                    "Window.this.log",
                    &[sciter_serde::to_value(&entry).unwrap()],
                )
                .unwrap_or_default();
            }
        }
    }
}

impl log::Log for Logger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        let entry: Entry = record.into();
        if record.target().contains("ukmm") && (self.debug || record.level() < LevelFilter::Debug) {
            if let Some(root) = self.root.get() {
                self.flush_queue();
                root.call_function(
                    "Window.this.log",
                    &[sciter_serde::to_value(&entry).unwrap()],
                )
                .unwrap_or_default();
            } else {
                self.queue.lock().push(entry);
            }
        }
        if self.enabled(record.metadata()) {
            self.inner.log(record);
        }
    }

    fn flush(&self) {
        self.flush_queue();
        self.inner.flush();
    }
}
