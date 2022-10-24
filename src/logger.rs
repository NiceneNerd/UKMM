use crate::gui::Message;
use log::{LevelFilter, Record};
use once_cell::sync::{Lazy, OnceCell};
use parking_lot::Mutex;
use serde::Serialize;
use std::{ops::Deref, sync::atomic::AtomicBool};

pub static LOGGER: Lazy<Logger> = Lazy::new(|| Logger {
    inner: env_logger::builder().build(),
    debug: std::env::args().any(|arg| &arg == "--debug").into(),
    queue: Mutex::new(vec![]),
    sender: OnceCell::new(),
});

pub fn init() {
    log::set_logger(LOGGER.deref()).unwrap();
    let level = log::max_level();
    log::set_max_level(level.max(log::LevelFilter::Debug));
}

#[derive(Debug, Clone, Serialize)]
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
            args: format!("{:#?}", record.args()),
        }
    }
}

pub struct Logger {
    inner: env_logger::Logger,
    debug: AtomicBool,
    queue: Mutex<Vec<Entry>>,
    sender: OnceCell<flume::Sender<Message>>,
}

impl Logger {
    pub fn debug(&self) -> bool {
        self.debug.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_debug(&self, debug: bool) {
        self.debug
            .store(debug, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_sender(&self, sender: flume::Sender<Message>) {
        self.sender.set(sender).unwrap_or(());
        self.flush_queue();
    }

    pub fn flush_queue(&self) {
        if let Some(sender) = self.sender.get() {
            for entry in self.queue.lock().drain(..) {
                sender.send(Message::Log(entry)).unwrap();
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
        if record.target().starts_with("uk")
            && (self.debug() || record.level() < LevelFilter::Debug)
        {
            if let Some(sender) = self.sender.get() {
                sender.send(Message::Log(entry)).unwrap();
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
