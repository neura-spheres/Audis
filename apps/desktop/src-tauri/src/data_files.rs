//! Browsing the files Audis has written.
//!
//! The frontend receives absolute paths and passes one back to open or reveal
//! it. That makes the path an untrusted input on the way back in, so
//! [`resolve_inside_root`] re-checks every path against the data root before
//! anything is opened. Canonicalising first means `..` segments, symlinks and
//! short 8.3 names cannot be used to escape.

use std::path::{Path, PathBuf};

use audis_common::{
    AppPaths, AudisError, DataCategory, DataCategoryGroup, DataFile, DataFileListing, Result,
};

/// Directories deeper than this are not walked.
///
/// Session folders are two or three levels at most. The cap stops a symlink
/// loop or a pathological tree from hanging the UI.
const MAX_DEPTH: usize = 8;

/// Files returned per category.
///
/// Logs and recordings can pile up, and neither the IPC payload nor the list UI
/// benefits from thousands of rows.
const MAX_FILES_PER_CATEGORY: usize = 500;

/// List every file under the data root, grouped by category.
pub fn list(paths: &AppPaths) -> Result<DataFileListing> {
    let root = paths.root().to_path_buf();
    let mut groups = Vec::new();
    let mut total_bytes = 0;
    let mut total_files = 0;

    for category in DataCategory::ALL {
        let dir = root.join(category.dir_name());
        let mut files = Vec::new();
        collect(&dir, &root, category, 0, &mut files);

        // Newest first: the file a user wants is almost always the last one
        // written.
        files.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
        files.truncate(MAX_FILES_PER_CATEGORY);

        let group_bytes: u64 = files.iter().map(|file| file.size_bytes).sum();
        total_bytes += group_bytes;
        total_files += files.len();

        groups.push(DataCategoryGroup {
            category,
            label: category.label().to_owned(),
            path: dir.display().to_string(),
            files,
            total_bytes: group_bytes,
        });
    }

    Ok(DataFileListing {
        root: root.display().to_string(),
        groups,
        total_bytes,
        total_files,
    })
}

/// Walk `dir`, appending files to `out`.
///
/// Unreadable entries are skipped rather than failing the whole listing: one
/// locked file should not blank the entire view.
fn collect(dir: &Path, root: &Path, category: DataCategory, depth: usize, out: &mut Vec<DataFile>) {
    if depth > MAX_DEPTH {
        return;
    }

    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(metadata) = entry.metadata() else {
            continue;
        };

        if metadata.is_dir() {
            collect(&path, root, category, depth + 1, out);
            continue;
        }

        if !metadata.is_file() {
            continue;
        }

        let relative_path = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string();

        let modified = metadata.modified().ok().map(|time| {
            let stamp: chrono::DateTime<chrono::Utc> = time.into();
            stamp.to_rfc3339()
        });

        out.push(DataFile {
            path: path.display().to_string(),
            relative_path,
            name: entry.file_name().to_string_lossy().into_owned(),
            size_bytes: metadata.len(),
            modified,
            category,
        });
    }
}

/// Resolve a caller-supplied path and confirm it is a real file inside the data
/// root.
///
/// Both sides are canonicalised before comparison, so `..`, a symlink out of
/// the tree, or a short name cannot escape.
pub fn resolve_inside_root(paths: &AppPaths, candidate: &str) -> Result<PathBuf> {
    if candidate.trim().is_empty() {
        return Err(AudisError::InvalidArgument {
            field: "path".to_owned(),
            detail: "path is empty".to_owned(),
        });
    }

    let root = paths
        .root()
        .canonicalize()
        .map_err(|source| AudisError::Io {
            path: paths.root().to_path_buf(),
            detail: "could not resolve the Audis data folder".to_owned(),
            source,
        })?;

    let resolved = PathBuf::from(candidate)
        .canonicalize()
        .map_err(|source| AudisError::Io {
            path: PathBuf::from(candidate),
            detail: "that file could not be found".to_owned(),
            source,
        })?;

    if !resolved.starts_with(&root) {
        // Deliberately vague to the user; the detail goes to the log instead.
        tracing::warn!(
            requested = %resolved.display(),
            "refused a path outside the Audis data folder"
        );
        return Err(AudisError::InvalidArgument {
            field: "path".to_owned(),
            detail: "path is outside the Audis data folder".to_owned(),
        });
    }

    Ok(resolved)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    fn temp_paths() -> (tempfile::TempDir, AppPaths) {
        let dir = tempfile::tempdir().expect("temp dir");
        let paths = AppPaths::rooted_at(dir.path().join("Audis"));
        paths.ensure_created().expect("create tree");
        (dir, paths)
    }

    #[test]
    fn listing_an_empty_tree_returns_every_category_with_no_files() {
        let (_guard, paths) = temp_paths();

        let listing = list(&paths).expect("list");

        assert_eq!(listing.groups.len(), DataCategory::ALL.len());
        assert_eq!(listing.total_files, 0);
        assert_eq!(listing.total_bytes, 0);
        assert!(listing.groups.iter().all(|group| group.files.is_empty()));
    }

    #[test]
    fn listing_finds_files_and_sums_their_sizes() {
        let (_guard, paths) = temp_paths();
        std::fs::write(paths.logs_dir().join("audis.log.2026-01-01"), "hello").expect("write log");
        std::fs::write(paths.exports_dir().join("meeting.txt"), "abc").expect("write export");

        let listing = list(&paths).expect("list");

        assert_eq!(listing.total_files, 2);
        assert_eq!(listing.total_bytes, 8);

        let logs = listing
            .groups
            .iter()
            .find(|group| group.category == DataCategory::Logs)
            .expect("logs group");
        assert_eq!(logs.files.len(), 1);
        assert_eq!(logs.files[0].name, "audis.log.2026-01-01");
        assert_eq!(logs.files[0].size_bytes, 5);
        assert_eq!(logs.files[0].relative_path, r"logs\audis.log.2026-01-01");
    }

    #[test]
    fn listing_walks_nested_session_folders() {
        let (_guard, paths) = temp_paths();
        let session = paths.sessions_dir().join("abc").join("attachments");
        std::fs::create_dir_all(&session).expect("create session dir");
        std::fs::write(session.join("note.txt"), "x").expect("write");

        let listing = list(&paths).expect("list");

        let sessions = listing
            .groups
            .iter()
            .find(|group| group.category == DataCategory::Sessions)
            .expect("sessions group");
        assert_eq!(sessions.files.len(), 1);
        assert_eq!(sessions.files[0].name, "note.txt");
    }

    #[test]
    fn a_file_inside_the_root_resolves() {
        let (_guard, paths) = temp_paths();
        let file = paths.logs_dir().join("audis.log");
        std::fs::write(&file, "x").expect("write");

        let resolved = resolve_inside_root(&paths, &file.display().to_string()).expect("resolve");

        assert!(resolved.ends_with("audis.log"));
    }

    /// The important one: a path from the frontend must never reach outside the
    /// data folder, however it is spelled.
    #[test]
    fn traversal_out_of_the_root_is_refused() {
        let (guard, paths) = temp_paths();
        let outside = guard.path().join("secret.txt");
        std::fs::write(&outside, "private").expect("write");

        let attempts = [
            outside.display().to_string(),
            paths
                .logs_dir()
                .join("..")
                .join("..")
                .join("secret.txt")
                .display()
                .to_string(),
            r"C:\Windows\System32\drivers\etc\hosts".to_owned(),
        ];

        for attempt in attempts {
            let result = resolve_inside_root(&paths, &attempt);
            assert!(result.is_err(), "should have refused: {attempt}");
        }
    }

    #[test]
    fn an_empty_path_is_refused() {
        let (_guard, paths) = temp_paths();
        assert!(resolve_inside_root(&paths, "   ").is_err());
    }
}
