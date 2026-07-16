//! Loading and saving user settings.

use std::path::PathBuf;
use std::sync::Mutex;

use audis_common::{AppPaths, AudisError, Result, Settings};

const FILE_NAME: &str = "settings.json";

/// Settings held in memory, with the file as the durable copy.
pub struct SettingsStore {
    path: PathBuf,
    temp_dir: PathBuf,
    current: Mutex<Settings>,
}

impl SettingsStore {
    /// Load settings from disk, falling back to defaults.
    pub fn load(paths: &AppPaths) -> Self {
        let path = paths.root().join(FILE_NAME);

        let current = match std::fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str::<Settings>(&contents) {
                Ok(settings) => settings,
                Err(error) => {
                    tracing::warn!(%error, "settings file could not be parsed; using defaults");
                    Settings::default()
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Settings::default(),
            Err(error) => {
                tracing::warn!(%error, "settings file could not be read; using defaults");
                Settings::default()
            }
        };

        Self {
            path,
            temp_dir: paths.temp_dir(),
            current: Mutex::new(current),
        }
    }

    /// A copy of the current settings.
    pub fn get(&self) -> Settings {
        self.current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    /// Replace the settings and persist them.
    pub fn set(&self, settings: Settings) -> Result<Settings> {
        self.write(&settings)?;

        let mut guard = self
            .current
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = settings.clone();

        Ok(settings)
    }

    fn write(&self, settings: &Settings) -> Result<()> {
        let json =
            serde_json::to_string_pretty(settings).map_err(|source| AudisError::Serialization {
                context: FILE_NAME.to_owned(),
                source,
            })?;

        std::fs::create_dir_all(&self.temp_dir).map_err(|source| AudisError::Io {
            path: self.temp_dir.clone(),
            detail: "could not prepare the temporary directory".to_owned(),
            source,
        })?;

        let staging = self
            .temp_dir
            .join(format!("{FILE_NAME}.{}", std::process::id()));

        std::fs::write(&staging, json).map_err(|source| AudisError::Io {
            path: staging.clone(),
            detail: "could not write settings".to_owned(),
            source,
        })?;

        std::fs::rename(&staging, &self.path).map_err(|source| AudisError::Io {
            path: self.path.clone(),
            detail: "could not save settings".to_owned(),
            source,
        })
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use audis_common::settings::{CloseBehavior, ThemePreference};

    fn temp_paths() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths::rooted_at(dir.path().join("Audis"));
        paths.ensure_created().expect("create tree");
        (dir, paths)
    }

    #[test]
    fn first_run_uses_defaults_without_a_file() {
        let (_guard, paths) = temp_paths();
        let store = SettingsStore::load(&paths);
        assert_eq!(store.get(), Settings::default());
    }

    #[test]
    fn settings_survive_a_reload() {
        let (_guard, paths) = temp_paths();

        let store = SettingsStore::load(&paths);
        let mut settings = store.get();
        settings.general.theme = ThemePreference::Dark;
        settings.general.close_behavior = CloseBehavior::Quit;
        store.set(settings).expect("save");

        let reloaded = SettingsStore::load(&paths);
        assert_eq!(reloaded.get().general.theme, ThemePreference::Dark);
        assert_eq!(reloaded.get().general.close_behavior, CloseBehavior::Quit);
    }

    /// A corrupt file must not stop Audis from starting, and must not be
    #[test]
    fn a_corrupt_file_falls_back_to_defaults_and_is_left_on_disk() {
        let (_guard, paths) = temp_paths();
        let file = paths.root().join(FILE_NAME);
        std::fs::write(&file, "{ not json").expect("write corrupt file");

        let store = SettingsStore::load(&paths);

        assert_eq!(store.get(), Settings::default());
        assert_eq!(
            std::fs::read_to_string(&file).expect("read"),
            "{ not json",
            "the unreadable file must be preserved for recovery"
        );
    }

    #[test]
    fn saving_leaves_no_staging_file_behind() {
        let (_guard, paths) = temp_paths();
        let store = SettingsStore::load(&paths);

        store.set(Settings::default()).expect("save");

        let leftovers: Vec<_> = std::fs::read_dir(paths.temp_dir())
            .expect("read temp")
            .filter_map(|entry| entry.ok())
            .collect();
        assert!(leftovers.is_empty(), "staging file was not renamed away");
    }
}
