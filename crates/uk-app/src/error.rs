//! Typed errors for host/open operations, so frontends can branch on variants
//! instead of matching English error strings.

/// Errors with a stable, matchable identity.
#[derive(Debug, Clone, thiserror::Error, serde::Serialize, serde::Deserialize)]
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

impl AppError {
    /// Converts an anyhow error, preserving a typed `AppError` when the chain
    /// contains one.
    pub fn from_anyhow(e: anyhow::Error) -> Self {
        e.downcast::<AppError>()
            .unwrap_or_else(|e| Self::Other(format!("{e:?}")))
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::from_anyhow(e)
    }
}
