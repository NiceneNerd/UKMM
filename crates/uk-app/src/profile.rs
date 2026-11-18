//! A validated profile name. Profiles are directories under the platform's
//! `profiles` folder, so names must never escape it.

use std::fmt;

use anyhow_ext::{Result, bail};

/// A profile name that is safe to join onto the profiles directory.
/// Construct user-supplied names only via [`ProfileName::parse`].
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct ProfileName(String);

impl ProfileName {
    /// Validates a user-supplied profile name: rejects empty names, `.`,
    /// `..`, path separators, and control characters.
    pub fn parse(name: impl AsRef<str>) -> Result<Self> {
        let name = name.as_ref();
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.contains(['/', '\\'])
            || name.chars().any(|c| c.is_control())
        {
            bail!("Invalid profile name: {name:?}");
        }
        Ok(Self(name.to_owned()))
    }

    /// Wraps an existing on-disk profile name without validation. Only for
    /// names read from disk; user input must go through [`ProfileName::parse`].
    pub fn from_existing(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    #[inline(always)]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[inline(always)]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl fmt::Display for ProfileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl AsRef<str> for ProfileName {
    #[inline(always)]
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_names_are_accepted() {
        for name in ["Default", "my profile 2", "100% Orange Juice", "üñïçødé"] {
            assert_eq!(ProfileName::parse(name).unwrap().as_str(), name);
        }
    }

    #[test]
    fn dangerous_names_are_rejected() {
        for name in [
            "",
            ".",
            "..",
            "a/b",
            "a\\b",
            "../profiles",
            "a\0b",
            "a\nb",
            "/etc",
        ] {
            assert!(
                ProfileName::parse(name).is_err(),
                "{name:?} must be rejected"
            );
        }
    }
}
