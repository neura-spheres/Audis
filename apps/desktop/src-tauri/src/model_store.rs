//! Installing and removing local speech models.
//!
//! Downloads stream to a temporary file and are renamed into place only once
//! complete. A half-downloaded model that looked installed would fail at the
//! worst possible moment, when the user starts a session.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use audis_common::{AppPaths, AudisError, InstalledModel, ModelDownloadProgress, ModelId, Result};
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};

/// Event channel for download progress.
pub const MODEL_PROGRESS_EVENT: &str = "audis://model/progress";

/// How often progress is reported, in bytes.
///
/// Emitting per chunk would flood the WebView with thousands of events for a
/// 1.5 GB model and stall the UI it is trying to update.
const PROGRESS_INTERVAL_BYTES: u64 = 2 * 1024 * 1024;

/// Tracks the download in flight, if any.
#[derive(Default)]
pub struct ModelStore {
    downloading: Mutex<Option<ModelId>>,
    cancel: std::sync::Arc<AtomicBool>,
}

impl ModelStore {
    /// Every model, with whether it is on disk.
    pub fn list(&self, paths: &AppPaths) -> Vec<InstalledModel> {
        ModelId::ALL
            .iter()
            .map(|id| {
                let info = id.info();
                let path = paths.models_dir().join(&info.file_name);
                let installed_bytes = std::fs::metadata(&path).ok().map(|meta| meta.len());

                InstalledModel {
                    // A zero-byte file is a failed download, not an install.
                    installed: installed_bytes.is_some_and(|bytes| bytes > 0),
                    installed_bytes,
                    info,
                }
            })
            .collect()
    }

    /// Path to a model, if it is installed and usable.
    pub fn path_if_installed(&self, paths: &AppPaths, id: ModelId) -> Option<std::path::PathBuf> {
        let path = paths.models_dir().join(id.file_name());
        match std::fs::metadata(&path) {
            Ok(meta) if meta.len() > 0 => Some(path),
            _ => None,
        }
    }

    /// True while a download is running.
    pub fn is_downloading(&self) -> bool {
        self.downloading
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_some()
    }

    /// Ask the running download to stop.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Download a model, reporting progress on [`MODEL_PROGRESS_EVENT`].
    pub async fn install(&self, app: AppHandle, paths: AppPaths, id: ModelId) -> Result<()> {
        {
            let mut guard = self
                .downloading
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            if guard.is_some() {
                return Err(AudisError::InvalidArgument {
                    field: "model".to_owned(),
                    detail: "another model is already downloading".to_owned(),
                });
            }
            *guard = Some(id);
        }
        self.cancel.store(false, Ordering::Relaxed);

        let result = self.download(&app, &paths, id).await;

        {
            let mut guard = self
                .downloading
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            *guard = None;
        }

        if let Err(error) = &result {
            let _ = app.emit(
                MODEL_PROGRESS_EVENT,
                ModelDownloadProgress {
                    id,
                    downloaded_bytes: 0,
                    total_bytes: None,
                    done: false,
                    error: Some(error.to_user_facing().explanation),
                },
            );
        }

        result
    }

    async fn download(&self, app: &AppHandle, paths: &AppPaths, id: ModelId) -> Result<()> {
        let info = id.info();
        let models_dir = paths.models_dir();
        let destination = models_dir.join(&info.file_name);

        std::fs::create_dir_all(&models_dir).map_err(|source| AudisError::Io {
            path: models_dir.clone(),
            detail: "could not create the models folder".to_owned(),
            source,
        })?;

        // Staged in the models folder rather than the temp folder, so the
        // rename below stays on one volume and therefore stays atomic.
        let staging = models_dir.join(format!("{}.partial", info.file_name));

        let response =
            reqwest::get(&info.url)
                .await
                .map_err(|error| AudisError::Configuration {
                    detail: format!("could not reach the model host: {error}"),
                })?;

        if !response.status().is_success() {
            return Err(AudisError::Configuration {
                detail: format!("the model host returned {}", response.status()),
            });
        }

        // The server's length, not the catalogue's: the catalogue figure is for
        // display and may drift when a model is republished.
        let total_bytes = response.content_length();

        let mut file = std::fs::File::create(&staging).map_err(|source| AudisError::Io {
            path: staging.clone(),
            detail: "could not create the download file".to_owned(),
            source,
        })?;

        let mut stream = response.bytes_stream();
        let mut downloaded: u64 = 0;
        let mut last_reported: u64 = 0;

        while let Some(chunk) = stream.next().await {
            if self.cancel.load(Ordering::Relaxed) {
                drop(file);
                let _ = std::fs::remove_file(&staging);
                return Err(AudisError::InvalidArgument {
                    field: "model".to_owned(),
                    detail: "the download was cancelled".to_owned(),
                });
            }

            let chunk = chunk.map_err(|error| AudisError::Configuration {
                detail: format!("the download was interrupted: {error}"),
            })?;

            use std::io::Write;
            file.write_all(&chunk).map_err(|source| AudisError::Io {
                path: staging.clone(),
                detail: "could not write the model to disk".to_owned(),
                source,
            })?;

            downloaded += chunk.len() as u64;

            if downloaded - last_reported >= PROGRESS_INTERVAL_BYTES {
                last_reported = downloaded;
                let _ = app.emit(
                    MODEL_PROGRESS_EVENT,
                    ModelDownloadProgress {
                        id,
                        downloaded_bytes: downloaded,
                        total_bytes,
                        done: false,
                        error: None,
                    },
                );
            }
        }

        file.sync_all().map_err(|source| AudisError::Io {
            path: staging.clone(),
            detail: "could not finish writing the model".to_owned(),
            source,
        })?;
        drop(file);

        // A truncated file that is renamed into place would look installed and
        // then fail when a session starts, which is the worst time to find out.
        if let Some(expected) = total_bytes
            && downloaded != expected
        {
            let _ = std::fs::remove_file(&staging);
            return Err(AudisError::Configuration {
                detail: format!("the download was incomplete: {downloaded} of {expected} bytes"),
            });
        }

        std::fs::rename(&staging, &destination).map_err(|source| AudisError::Io {
            path: destination.clone(),
            detail: "could not save the model".to_owned(),
            source,
        })?;

        tracing::info!(?id, bytes = downloaded, "model installed");

        let _ = app.emit(
            MODEL_PROGRESS_EVENT,
            ModelDownloadProgress {
                id,
                downloaded_bytes: downloaded,
                total_bytes,
                done: true,
                error: None,
            },
        );

        Ok(())
    }

    /// Delete an installed model.
    pub fn remove(&self, paths: &AppPaths, id: ModelId) -> Result<()> {
        let path = paths.models_dir().join(id.file_name());

        match std::fs::remove_file(&path) {
            Ok(()) => {
                tracing::info!(?id, "model removed");
                Ok(())
            }
            // Already gone is the outcome the user wanted.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(AudisError::Io {
                path,
                detail: "could not remove the model".to_owned(),
                source,
            }),
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn temp_paths() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths::rooted_at(dir.path().join("Audis"));
        paths.ensure_created().expect("tree");
        (dir, paths)
    }

    #[test]
    fn nothing_is_installed_on_a_fresh_machine() {
        let (_guard, paths) = temp_paths();
        let store = ModelStore::default();

        let listed = store.list(&paths);

        assert_eq!(listed.len(), ModelId::ALL.len());
        assert!(listed.iter().all(|model| !model.installed));
        assert!(
            store
                .path_if_installed(&paths, ModelId::WhisperBase)
                .is_none()
        );
    }

    #[test]
    fn a_present_file_reports_as_installed_with_its_real_size() {
        let (_guard, paths) = temp_paths();
        let store = ModelStore::default();
        std::fs::write(
            paths.models_dir().join(ModelId::WhisperBase.file_name()),
            vec![0u8; 1234],
        )
        .expect("write model");

        let base = store
            .list(&paths)
            .into_iter()
            .find(|model| model.info.id == ModelId::WhisperBase)
            .expect("base in catalogue");

        assert!(base.installed);
        assert_eq!(base.installed_bytes, Some(1234));
        assert!(
            store
                .path_if_installed(&paths, ModelId::WhisperBase)
                .is_some()
        );
    }

    /// A zero-byte file is what a failed download leaves behind. Treating it as
    /// installed would fail at session start instead of at install time.
    #[test]
    fn an_empty_file_is_not_an_installed_model() {
        let (_guard, paths) = temp_paths();
        let store = ModelStore::default();
        std::fs::write(
            paths.models_dir().join(ModelId::WhisperTiny.file_name()),
            b"",
        )
        .expect("write empty");

        let tiny = store
            .list(&paths)
            .into_iter()
            .find(|model| model.info.id == ModelId::WhisperTiny)
            .expect("tiny in catalogue");

        assert!(!tiny.installed, "an empty file must not count as installed");
        assert!(
            store
                .path_if_installed(&paths, ModelId::WhisperTiny)
                .is_none()
        );
    }

    #[test]
    fn removing_a_model_deletes_it_and_is_idempotent() {
        let (_guard, paths) = temp_paths();
        let store = ModelStore::default();
        let path = paths.models_dir().join(ModelId::WhisperBase.file_name());
        std::fs::write(&path, vec![0u8; 10]).expect("write");

        store.remove(&paths, ModelId::WhisperBase).expect("remove");
        assert!(!path.exists());

        // Removing something already gone is success, not an error.
        store
            .remove(&paths, ModelId::WhisperBase)
            .expect("second remove must succeed");
    }

    #[test]
    fn a_partial_file_does_not_look_like_an_install() {
        let (_guard, paths) = temp_paths();
        let store = ModelStore::default();
        std::fs::write(
            paths
                .models_dir()
                .join(format!("{}.partial", ModelId::WhisperBase.file_name())),
            vec![0u8; 500],
        )
        .expect("write partial");

        assert!(
            store
                .path_if_installed(&paths, ModelId::WhisperBase)
                .is_none(),
            "a .partial file must never be mistaken for the model"
        );
    }
}
