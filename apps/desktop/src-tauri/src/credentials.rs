//! API keys, in the Windows Credential Manager.

use audis_common::{AudisError, Result, identity, providers::ProviderId};
use keyring::Entry;

/// Service name under which entries are filed in the keystore.
fn service() -> String {
    identity::BUNDLE_ID.to_owned()
}

fn entry(provider: ProviderId) -> Result<Entry> {
    Entry::new(&service(), &provider.credential_ref()).map_err(|error| AudisError::Configuration {
        detail: format!("the credential store is unavailable: {error}"),
    })
}

/// Save a key, replacing any existing one.
pub fn set_key(provider: ProviderId, key: &str) -> Result<()> {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return Err(AudisError::InvalidArgument {
            field: "key".to_owned(),
            detail: "the key is empty".to_owned(),
        });
    }

    entry(provider)?
        .set_password(trimmed)
        .map_err(|error| AudisError::Configuration {
            detail: format!("could not save the key: {error}"),
        })?;

    tracing::info!(
        provider = provider.slug(),
        "API key saved to the credential store"
    );
    Ok(())
}

/// Fetch a key for a provider call.
pub fn get_key(provider: ProviderId) -> Result<Option<String>> {
    match entry(provider)?.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(AudisError::Configuration {
            detail: format!("could not read the key: {error}"),
        }),
    }
}

/// Whether a key is saved, without reading it.
pub fn has_key(provider: ProviderId) -> bool {
    matches!(get_key(provider), Ok(Some(_)))
}

/// Delete a key.
pub fn delete_key(provider: ProviderId) -> Result<()> {
    match entry(provider)?.delete_credential() {
        Ok(()) => {
            tracing::info!(provider = provider.slug(), "API key deleted");
            Ok(())
        }
        Err(keyring::Error::NoEntry) => Ok(()),
        Err(error) => Err(AudisError::Configuration {
            detail: format!("could not delete the key: {error}"),
        }),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The keystore is one OS-wide resource shared by the whole machine, and
    static STORE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Take exclusive use of the test credential slot.
    fn exclusive() -> std::sync::MutexGuard<'static, ()> {
        STORE_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// These tests touch the real keystore. They use a provider slug that no
    fn store_available() -> bool {
        Entry::new("ai.neura.audis.test", "probe")
            .and_then(|entry| entry.set_password("probe"))
            .is_ok()
    }

    #[test]
    fn a_key_round_trips_and_can_be_deleted() {
        let _guard = exclusive();
        if !store_available() {
            eprintln!("skipping: no OS credential store on this machine");
            return;
        }

        let provider = ProviderId::OpenAiCompatible;
        let _ = delete_key(provider);

        assert!(!has_key(provider), "should start empty");

        set_key(provider, "sk-test-value-12345").expect("save");
        assert!(has_key(provider));
        assert_eq!(
            get_key(provider).expect("read"),
            Some("sk-test-value-12345".to_owned())
        );

        delete_key(provider).expect("delete");
        assert!(!has_key(provider), "should be gone after delete");

        delete_key(provider).expect("second delete must succeed");
    }

    #[test]
    fn whitespace_is_trimmed_so_a_pasted_key_still_works() {
        let _guard = exclusive();
        if !store_available() {
            return;
        }

        let provider = ProviderId::OpenAiCompatible;
        let _ = delete_key(provider);

        set_key(provider, "  sk-padded-key  \n").expect("save");
        assert_eq!(
            get_key(provider).expect("read"),
            Some("sk-padded-key".to_owned())
        );

        let _ = delete_key(provider);
    }

    #[test]
    fn an_empty_key_is_refused_rather_than_stored() {
        let result = set_key(ProviderId::Gemini, "   ");
        assert!(result.is_err(), "an empty key must not be saved");
    }

    #[test]
    fn a_missing_key_reads_as_none_not_an_error() {
        let _guard = exclusive();
        if !store_available() {
            return;
        }

        let provider = ProviderId::OpenAiCompatible;
        let _ = delete_key(provider);

        assert_eq!(get_key(provider).expect("read must succeed"), None);
        assert!(!has_key(provider));
    }
}
