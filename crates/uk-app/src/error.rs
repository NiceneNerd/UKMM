//! Typed errors for host/open operations, so frontends can branch on variants
//! instead of matching English error strings.

/// Errors with a stable, matchable identity.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    /// The mod targets a different platform than the current mode.
    #[error("Mod is for {0}, current mode is {1}")]
    WrongPlatform(String, String),
    /// A required master mod (dependency) is not installed.
    #[error("Could not find required mod dependency {name} (version {version})")]
    MissingMasters { name: String, version: String },
    /// No game dump is configured for the current platform.
    #[error("No dump available for current platform")]
    DumpMissing,
    /// The offered update has no asset for this platform.
    #[error("No matching platform for update")]
    UpdateUnavailable,
    /// Anything else, kept as its debug chain.
    #[error("{0}")]
    Other(String),
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Other(format!("{e:?}"))
    }
}
