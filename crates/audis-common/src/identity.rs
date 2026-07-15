//! Product identity constants.
//!
//! Defined once here so the installer, tray, About page and update metadata
//! can never disagree about what this application is called.

/// Company and product family.
pub const COMPANY: &str = "Neura Audis";

/// Application name, as shown to users.
pub const APP_NAME: &str = "Audis";

/// Publisher string used by the installer and Add/Remove Programs.
pub const PUBLISHER: &str = "Neura Audis";

/// Product tagline.
pub const TAGLINE: &str = "Hear more. Understand faster.";

/// Tauri/OS bundle identifier.
pub const BUNDLE_ID: &str = "ai.neura.audis";

/// Custom URL protocol scheme, without the `://` suffix.
pub const PROTOCOL_SCHEME: &str = "audis";

/// Prefix for every application event emitted over IPC.
pub const EVENT_PREFIX: &str = "audis://";

/// Prefix for environment variables that configure Audis.
pub const ENV_PREFIX: &str = "AUDIS_";

/// Structured-log target prefix.
pub const LOG_PREFIX: &str = "audis";

/// SQLite database filename.
pub const DATABASE_FILENAME: &str = "audis.db";

/// Parent folder under `%LOCALAPPDATA%`.
pub const DATA_VENDOR_DIR: &str = "NeuraAudis";

/// Application folder inside [`DATA_VENDOR_DIR`].
pub const DATA_APP_DIR: &str = "Audis";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_prefix_matches_protocol_scheme() {
        assert_eq!(EVENT_PREFIX, format!("{PROTOCOL_SCHEME}://"));
    }

    #[test]
    fn identity_values_are_populated() {
        for value in [
            COMPANY,
            APP_NAME,
            PUBLISHER,
            TAGLINE,
            BUNDLE_ID,
            PROTOCOL_SCHEME,
            DATABASE_FILENAME,
        ] {
            assert!(!value.is_empty());
        }
    }
}
