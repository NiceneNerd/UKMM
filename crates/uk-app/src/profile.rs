//! A validated profile name plus the profile store: listing profiles and
//! reading their `profile.yml` data, so frontends never do raw fs ops under
//! the profiles dir themselves.

use std::fmt;

use anyhow_ext::{Result, bail};
use uk_manager::{mods::Profile, settings::Settings};

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

/// Reads the profile store (`<storage>/<platform>/profiles/`): the list of
/// profiles on disk and their `profile.yml` data.
pub struct ProfileService<'a> {
    settings: &'a Settings,
}

impl<'a> ProfileService<'a> {
    pub fn new(settings: &'a Settings) -> Self {
        Self { settings }
    }

    /// All profile directories on disk. On-disk names are trusted as-is (only
    /// user-typed names go through [`ProfileName::parse`]).
    pub fn list(&self) -> Vec<ProfileName> {
        self.settings
            .profiles()
            .map(ProfileName::from_existing)
            .collect()
    }

    /// A profile's saved data, or `None` when its `profile.yml` is missing or
    /// malformed (the profile still shows up in [`ProfileService::list`]).
    pub fn read(&self, name: &ProfileName) -> Result<Option<Profile>> {
        let path = self
            .settings
            .profiles_dir()
            .join(name.as_str())
            .join("profile.yml");
        let Ok(text) = fs_err::read_to_string(&path) else {
            log::warn!("Failed to read profile data from {}", path.display());
            return Ok(None);
        };
        match serde_yaml::from_str(&text) {
            Ok(data) => Ok(Some(data)),
            Err(e) => {
                log::warn!("Failed to parse profile data from {}: {e}", path.display());
                Ok(None)
            }
        }
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

    fn temp_settings() -> (tempfile::TempDir, Settings) {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings::load_from(&dir.path().join("settings.yml"))
            .read()
            .clone();
        (dir, settings)
    }

    #[test]
    fn service_lists_and_reads_profiles() {
        let (_dir, settings) = temp_settings();
        let profiles_dir = settings.profiles_dir();
        fs_err::create_dir_all(profiles_dir.join("Default")).unwrap();
        fs_err::write(
            profiles_dir.join("Default").join("profile.yml"),
            serde_yaml::to_string(&Profile::default()).unwrap(),
        )
        .unwrap();
        // A profile dir without profile.yml: listed, but reads as None.
        fs_err::create_dir_all(profiles_dir.join("broken")).unwrap();

        let service = ProfileService::new(&settings);
        let list = service.list();
        let names: Vec<&str> = list.iter().map(|p| p.as_str()).collect();
        assert!(names.contains(&"Default"));
        assert!(names.contains(&"broken"));
        assert!(
            service
                .read(&ProfileName::from_existing("Default"))
                .unwrap()
                .is_some()
        );
        assert!(
            service
                .read(&ProfileName::from_existing("broken"))
                .unwrap()
                .is_none()
        );
        assert!(
            service
                .read(&ProfileName::from_existing("missing"))
                .unwrap()
                .is_none()
        );
    }
}
