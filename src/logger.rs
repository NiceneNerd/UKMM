use std::{
    io::Write,
    ops::Deref,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

use log::{LevelFilter, Record};
use parking_lot::Mutex;
use uk_util::{Lazy, OnceLock};

use crate::gui::Message;

pub static LOGGER: Lazy<Logger> = Lazy::new(|| {
    Logger {
        inner:  env_logger::builder().build(),
        debug:  std::env::args().any(|arg| &arg == "--debug").into(),
        queue:  Mutex::new(vec![]),
        sender: OnceLock::new(),
        record: Mutex::new(vec![]),
        file:   OnceLock::new(),
    }
});

pub fn init() {
    log::set_logger(LOGGER.deref()).unwrap();
    let level = LOGGER.inner.filter();
    log::set_max_level(level.max(log::LevelFilter::Debug));
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub args: String,
}

impl From<&Record<'_>> for Entry {
    fn from(record: &Record) -> Self {
        Self {
            timestamp: astrolabe::DateTime::now().format("y-MM-dd h:mm:ss"),
            level: record.level().to_string(),
            target: record.target().to_string(),
            args: format!("{:?}", record.args()),
        }
    }
}

pub struct Logger {
    inner:  env_logger::Logger,
    debug:  AtomicBool,
    queue:  Mutex<Vec<Entry>>,
    sender: OnceLock<flume::Sender<Message>>,
    record: Mutex<Vec<Entry>>,
    file:   OnceLock<PathBuf>,
}

impl Drop for Logger {
    fn drop(&mut self) {
        self.save_log()
    }
}

impl Logger {
    pub fn save_log(&self) {
        if let Some(path) = self.file.get() {
            let mut file = if path.exists() {
                fs_err::OpenOptions::new().append(true).open(path)
            } else {
                fs_err::File::create(path)
            }
            .unwrap();

            for entry in self.record.lock().drain(..) {
                writeln!(file, "[{}] {} {}", entry.timestamp, entry.level, entry.args)
                    .unwrap_or(());
            }
        }
    }

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

    pub fn set_file(&self, file: PathBuf) {
        self.file.set(file).unwrap_or(());
    }

    pub fn log_path(&self) -> Option<&Path> {
        self.file.get().map(|f| f.as_path())
    }

    pub fn flush_queue(&self) {
        if let Some(sender) = self.sender.get() {
            let mut queue = self.queue.lock();
            if queue.len() > 1000 {
                queue.drain(..500).count();
            }
            for entry in queue.drain(..) {
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
            if !entry.args.starts_with("PROGRESS") {
                self.record.lock().push(entry.clone());
            }
            if let Some(sender) = self.sender.get() {
                sender.send(Message::Log(entry)).unwrap();
            } else {
                self.queue.lock().push(entry);
            }
            if self.enabled(record.metadata())
                && record
                    .args()
                    .as_str()
                    .map(|s| !s.starts_with("PROGRESS"))
                    .unwrap_or(true)
            {
                self.inner.log(record);
            }
        }
        if self.record.lock().len() > 30 {
            self.save_log();
        }
    }

    fn flush(&self) {
        self.flush_queue();
        self.inner.flush();
    }
}
