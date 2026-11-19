//! Host-environment interactions for frontends: HTTP, update checks,
//! GameBanana one-click IPC, `bcml:` URL-handler registration, and bare CLI
//! argument classification. None of this depends on a GUI toolkit.

pub mod args;
pub mod handlers;
pub mod http;
pub mod oneclick;
pub mod shell;
pub mod updates;
