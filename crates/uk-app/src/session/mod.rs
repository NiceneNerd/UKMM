//! The session: the single owner of mod state between the core
//! (`uk-manager`) and any frontend.
//!
//! Frontends dispatch [`Command`]s and read an immutable [`Snapshot`]; the
//! session answers with a stream of [`Event`]s. One mutating job runs at a
//! time on a worker thread; a panic in a job becomes
//! `Event::JobDone(Err(..))`, never a dead frontend.

mod command;
mod event;
mod handlers;
mod snapshot;
#[cfg(test)]
mod tests;

use std::{
    panic::AssertUnwindSafe,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use anyhow_ext::Result;
pub use command::{Command, InstallCandidate, OptionsPrompt};
pub use event::{Event, Inspection, MasterRef};
use parking_lot::RwLock;
use rustc_hash::FxHashMap;
pub use snapshot::{JobId, ModEntry, ModId, PendingSummary, Snapshot};
use uk_manager::{core::Manager, mods::Mod};
use uk_mod::Manifest;

use crate::{error::AppError, profile::ProfileName};

/// A session on top of a [`Manager`]: dispatch [`Command`]s, read the
/// [`Snapshot`], subscribe to [`Event`]s.
pub struct Session {
    core: Arc<Manager>,
    tx: flume::Sender<(JobId, Command, bool)>,
    subscribers: Arc<RwLock<Vec<flume::Sender<Event>>>>,
    snap: Arc<RwLock<Arc<Snapshot>>>,
    next_job: AtomicU64,
}

impl Session {
    /// Opens a session, seeding the working state from the manager's current
    /// profile.
    pub fn open(core: Arc<Manager>) -> Result<Session> {
        let (tx, rx) = flume::unbounded();
        let subscribers = Arc::new(RwLock::new(vec![]));
        let snap = Arc::new(RwLock::new(Arc::new(Snapshot {
            revision: 0,
            profile: ProfileName::from_existing("Default"),
            entries: vec![],
            pending: PendingSummary::default(),
            busy: None,
            progress: None,
            installing: 0,
        })));
        let mut state = State::new(core.clone(), subscribers.clone(), snap.clone());
        state.refresh_mods();
        state.publish();
        std::thread::Builder::new()
            .name("ukmm-session".into())
            .spawn(move || state.run(rx))?;
        Ok(Session {
            core,
            tx,
            subscribers,
            snap,
            next_job: AtomicU64::new(1),
        })
    }

    /// The manager this session runs on.
    pub fn core(&self) -> &Arc<Manager> {
        &self.core
    }

    /// The latest immutable snapshot.
    pub fn snapshot(&self) -> Arc<Snapshot> {
        self.snap.read().clone()
    }

    /// A new event receiver. Every receiver sees every event — the events
    /// are broadcast, not shared between receivers.
    pub fn subscribe(&self) -> flume::Receiver<Event> {
        let (tx, rx) = flume::unbounded();
        self.subscribers.write().push(tx);
        rx
    }

    /// Enqueues a blocking command and returns its job id: while it runs,
    /// [`Snapshot::busy`] is set and the frontend should show a busy state.
    /// If the worker has shut down the command is silently dropped.
    pub fn dispatch(&self, cmd: Command) -> JobId {
        self.dispatch_inner(cmd, false)
    }

    /// Enqueues a background command (e.g. a startup update check): it runs
    /// like any other job — events, revision bumps, `JobDone` — but never
    /// sets [`Snapshot::busy`], so the frontend stays interactive.
    pub fn dispatch_background(&self, cmd: Command) -> JobId {
        self.dispatch_inner(cmd, true)
    }

    fn dispatch_inner(&self, cmd: Command, background: bool) -> JobId {
        let job = JobId(self.next_job.fetch_add(1, Ordering::Relaxed));
        let _ = self.tx.send((job, cmd, background));
        job
    }
}

/// The worker's mutable state: the working mod list (load order), the
/// per-profile pending manifests, and job bookkeeping.
struct State {
    core: Arc<Manager>,
    mods: Vec<Mod>,
    dirty: FxHashMap<String, Manifest>,
    revision: u64,
    busy: Option<JobId>,
    progress: Option<String>,
    installing: usize,
    subscribers: Arc<RwLock<Vec<flume::Sender<Event>>>>,
    snap: Arc<RwLock<Arc<Snapshot>>>,
}

impl State {
    fn new(
        core: Arc<Manager>,
        subscribers: Arc<RwLock<Vec<flume::Sender<Event>>>>,
        snap: Arc<RwLock<Arc<Snapshot>>>,
    ) -> Self {
        // Every profile gets a pending entry up front, so lookups never fall
        // back to another profile or panic on an empty map.
        let dirty = core
            .settings()
            .profiles()
            .map(|p| (p.to_string(), Manifest::default()))
            .collect();
        Self {
            core,
            mods: vec![],
            dirty,
            revision: 0,
            busy: None,
            progress: None,
            installing: 0,
            subscribers,
            snap,
        }
    }

    fn run(mut self, rx: flume::Receiver<(JobId, Command, bool)>) {
        while let Ok((job, cmd, background)) = rx.recv() {
            self.emit(Event::JobStarted(job));
            if !background {
                self.busy = Some(job);
            }
            self.progress = None;
            self.publish();
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| self.handle(cmd)))
                .map_err(|e| AppError::Other(panic_message(e)))
                .and_then(|r| r.map_err(AppError::from_anyhow));
            if let Err(e) = &result {
                self.emit(Event::Error(e.clone()));
            }
            self.busy = None;
            self.progress = None;
            self.installing = 0;
            self.revision += 1;
            self.publish();
            self.emit(Event::Revision(self.revision));
            self.emit(Event::JobDone { job, result });
            if let Some(dump) = self.core.settings().dump() {
                dump.clear_cache();
            }
        }
    }

    /// Sends an event to every subscriber.
    fn emit(&self, ev: Event) {
        for sender in self.subscribers.read().iter() {
            let _ = sender.send(ev.clone());
        }
    }

    fn publish(&self) {
        *self.snap.write() = Arc::new(self.build_snapshot());
    }

    fn build_snapshot(&self) -> Snapshot {
        let profile = self.current_profile_name();
        let pending = self
            .dirty
            .get(profile.as_str())
            .cloned()
            .map(|manifest| PendingSummary { manifest })
            .unwrap_or_default();
        Snapshot {
            revision: self.revision,
            profile,
            entries: self.mods.iter().map(ModEntry::from_mod).collect(),
            pending,
            busy: self.busy,
            progress: self.progress.clone(),
            installing: self.installing,
        }
    }

    fn current_profile_name(&self) -> ProfileName {
        let mm = self.core.mod_manager();
        ProfileName::from_existing(mm.profile().key().to_string())
    }

    fn refresh_mods(&mut self) {
        self.mods = self.core.mod_manager().all_mods().collect();
    }

    fn dirty_current_mut(&mut self) -> &mut Manifest {
        let key = self.current_profile_name().into_string();
        self.dirty.entry(key).or_default()
    }

    fn emit_pending(&self) {
        let profile = self.current_profile_name();
        let files = self
            .dirty
            .get(profile.as_str())
            .map(manifest_files)
            .unwrap_or(0);
        self.emit(Event::PendingChanged { profile, files });
    }
}

/// Number of files a pending manifest touches.
fn manifest_files(m: &Manifest) -> usize {
    m.content_files.len() + m.aoc_files.len()
}

fn panic_message(e: Box<dyn std::any::Any + Send>) -> String {
    let msg = if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else if let Some(s) = e.downcast_ref::<&str>() {
        (*s).to_string()
    } else {
        "An unknown error occurred, check the log for possible details.".to_string()
    };
    format!("{msg}\n{}", std::backtrace::Backtrace::force_capture())
}
