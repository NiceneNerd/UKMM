use std::{
    fmt::Write,
    ops::Deref,
    path::{Path, PathBuf},
    sync::{Arc, LazyLock, OnceLock},
};

use log::Record;
use parking_lot::Mutex;

pub static LOGGER: LazyLock<Logger> = LazyLock::new(|| {
    Logger {
        text: Default::default(),
        record_buf: Arc::new(Mutex::new(String::with_capacity(512))),
        msg: Default::default(),
        inner: &egui_logger::EguiLogger,
        file: OnceLock::new(),
    }
});

pub fn init() {
    if let Ok(_) = log::set_logger(LOGGER.deref()) {
        log::set_max_level(log::LevelFilter::max());
    }
}

pub struct Logger {
    text: Arc<Mutex<String>>,
    record_buf: Arc<Mutex<String>>,
    msg: Arc<Mutex<Option<String>>>,
    inner: &'static egui_logger::EguiLogger,
    file: OnceLock<PathBuf>,
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
                fs_err::create_dir_all(path.parent().expect("Weird log path"))
                    .expect("Yikes, folder problem");
                fs_err::File::create(path)
            }
            .unwrap();
            use std::io::Write;
            let mut text = self.text.lock();
            file.write(text.as_bytes()).unwrap_or_default();
            text.clear();
        }
    }

    pub fn set_file(&self, mut file: PathBuf) {
        if file
            .metadata()
            .map(|m| m.len() > 1_048_576)
            .unwrap_or_default()
        {
            let file_num = file
                .file_stem()
                .expect("Bad log file stem")
                .to_str()
                .expect("Bad log file stem")
                .trim_start_matches("log")
                .trim_start_matches('.')
                .parse::<u8>()
                .unwrap_or_default()
                + 1;
            file.set_file_name(&format!("log.{}.txt", file_num));
        }
        self.file.set(file).unwrap_or(());
    }

    pub fn log_path(&self) -> Option<&Path> {
        self.file.get().map(|f| f.as_path())
    }

    pub fn get_progress(&self) -> Option<String> {
        self.msg.lock().clone()
    }
}

impl log::Log for Logger {
    #[inline]
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.inner.enabled(metadata)
    }

    fn log(&self, record: &Record) {
        if !record.target().starts_with("uk") {
            return;
        }
        let mut buf;
        let txt = match record.args().as_str() {
            Some(txt) => txt,
            None => {
                buf = self.record_buf.lock();
                buf.clear();
                let _ = buf.write_fmt(*record.args());
                buf.as_str()
            }
        };
        let progress_msg = txt.starts_with("PROGRESS");
        if record.level() == log::Level::Info || progress_msg {
            self.msg
                .lock()
                .replace(txt.trim_start_matches("PROGRESS").to_string());
        } else if txt == "CLEARPROGRESS" {
            *self.msg.lock() = None;
            return;
        }
        if !progress_msg {
            self.inner.log(record);
            let mut text = self.text.lock();
            writeln!(
                text,
                "[{}] {} {}",
                astrolabe::DateTime::now().format("y-MM-dd h:mm:ss"),
                record.level().as_str(),
                record.args()
            )
            .expect("Failed to write to log");
            if text.lines().count() > 1024 {
                drop(text);
                self.save_log();
            }
        }
    }

    #[inline]
    fn flush(&self) {
        self.inner.flush();
        self.save_log();
    }
}
