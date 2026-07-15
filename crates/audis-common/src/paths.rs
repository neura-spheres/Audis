//! Windows data directory layout.
//!
//! ```text
//! %LOCALAPPDATA%\NeuraAudis\Audis\
//!     database\audis.db
//!     sessions\  recordings\  models\  cache\
//!     logs\      updates\     exports\ temp\
//! ```
//!
//! Everything lives under `%LOCALAPPDATA%` rather than `%APPDATA%`: recordings
//! and models are large and machine-specific, and roaming them onto a domain
//! profile would be hostile. `%APPDATA%` is reserved for small roaming prefs.

use std::path::{Path, PathBuf};

use crate::error::{AudisError, Result};
use crate::identity::{DATA_APP_DIR, DATA_VENDOR_DIR, DATABASE_FILENAME, ENV_PREFIX};

/// Resolved on-disk locations for one Audis installation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    root: PathBuf,
}

impl AppPaths {
    /// Resolve the data root from the environment.
    ///
    /// `AUDIS_DATA_DIR` overrides the default, which supports portable installs
    /// and lets tests run against a scratch directory.
    pub fn discover() -> Result<Self> {
        if let Some(overridden) = std::env::var_os(format!("{ENV_PREFIX}DATA_DIR")) {
            let path = PathBuf::from(overridden);
            if path.as_os_str().is_empty() {
                return Err(AudisError::Configuration {
                    detail: format!("{ENV_PREFIX}DATA_DIR is set but empty"),
                });
            }
            return Ok(Self::rooted_at(path));
        }

        let dirs = directories::BaseDirs::new().ok_or_else(|| AudisError::Configuration {
            detail: "could not determine the local application data directory".to_owned(),
        })?;

        Ok(Self::rooted_at(
            dirs.data_local_dir()
                .join(DATA_VENDOR_DIR)
                .join(DATA_APP_DIR),
        ))
    }

    /// Build a layout rooted at an explicit directory.
    pub fn rooted_at(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The data root itself.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Directory holding the SQLite database and its WAL sidecars.
    pub fn database_dir(&self) -> PathBuf {
        self.root.join("database")
    }

    /// Full path to `audis.db`.
    pub fn database_file(&self) -> PathBuf {
        self.database_dir().join(DATABASE_FILENAME)
    }

    /// Per-session folders: transcripts, recovery journals, attachments.
    pub fn sessions_dir(&self) -> PathBuf {
        self.root.join("sessions")
    }

    /// Captured audio.
    pub fn recordings_dir(&self) -> PathBuf {
        self.root.join("recordings")
    }

    /// Downloaded local inference models.
    pub fn models_dir(&self) -> PathBuf {
        self.root.join("models")
    }

    /// Regenerable cache. Safe to delete.
    pub fn cache_dir(&self) -> PathBuf {
        self.root.join("cache")
    }

    /// Rolling structured logs.
    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    /// Staged updater artifacts.
    pub fn updates_dir(&self) -> PathBuf {
        self.root.join("updates")
    }

    /// User-facing exports.
    pub fn exports_dir(&self) -> PathBuf {
        self.root.join("exports")
    }

    /// Scratch space for write-then-rename. Kept on the data volume so the
    /// rename stays atomic.
    pub fn temp_dir(&self) -> PathBuf {
        self.root.join("temp")
    }

    /// Every directory Audis expects to exist.
    pub fn all_dirs(&self) -> Vec<PathBuf> {
        vec![
            self.root.clone(),
            self.database_dir(),
            self.sessions_dir(),
            self.recordings_dir(),
            self.models_dir(),
            self.cache_dir(),
            self.logs_dir(),
            self.updates_dir(),
            self.exports_dir(),
            self.temp_dir(),
        ]
    }

    /// Create the full directory tree. Idempotent.
    pub fn ensure_created(&self) -> Result<()> {
        for dir in self.all_dirs() {
            std::fs::create_dir_all(&dir).map_err(|source| AudisError::Io {
                path: dir.clone(),
                detail: "could not create Audis data directory".to_owned(),
                source,
            })?;
        }
        Ok(())
    }

    /// Directory for one session's files.
    ///
    /// Takes a `Uuid` rather than a string so a caller cannot smuggle `../`
    /// into the path.
    pub fn session_dir(&self, session_id: uuid::Uuid) -> PathBuf {
        self.sessions_dir().join(session_id.to_string())
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn layout_matches_the_documented_tree() {
        let paths = AppPaths::rooted_at(r"C:\data\NeuraAudis\Audis");

        assert_eq!(
            paths.database_file(),
            PathBuf::from(r"C:\data\NeuraAudis\Audis\database\audis.db")
        );
        assert_eq!(
            paths.recordings_dir(),
            PathBuf::from(r"C:\data\NeuraAudis\Audis\recordings")
        );
    }

    #[test]
    fn ensure_created_builds_every_directory_and_is_idempotent() {
        let temp = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths::rooted_at(temp.path().join("Audis"));

        paths.ensure_created().expect("first create");
        paths.ensure_created().expect("second create must not fail");

        for dir in paths.all_dirs() {
            assert!(dir.is_dir(), "expected directory: {}", dir.display());
        }
    }

    #[test]
    fn session_dir_is_confined_to_the_sessions_directory() {
        let paths = AppPaths::rooted_at(r"C:\data\Audis");
        let id = uuid::Uuid::new_v4();

        let dir = paths.session_dir(id);

        assert!(dir.starts_with(paths.sessions_dir()));
        assert_eq!(dir.file_name().unwrap().to_string_lossy(), id.to_string());
    }

    #[test]
    fn temp_dir_shares_the_root_volume() {
        let paths = AppPaths::rooted_at(r"C:\data\Audis");
        assert!(paths.temp_dir().starts_with(paths.root()));
    }
}
