//! Looking for new versions on GitHub Releases.
//!
//! Audis publishes every build as a GitHub release, tagged by semver:
//!
//! - `v1.2.3` for a finished release
//! - `v1.2.3-beta.1` for a beta, published as a GitHub pre-release
//!
//! Semver orders those the way people expect — `1.2.0-beta.1` precedes `1.2.0` —
//! so a beta tester is offered the finished release once it lands, rather than
//! being stranded on the pre-release.
//!
//! Nothing is installed unless it carries a signature made with the Audis
//! private key. That check is what makes downloading and running an installer
//! safe at all: without it, anyone able to answer for GitHub could hand the app
//! an executable of their choosing. The public half is compiled into the binary;
//! the private half never leaves its owner.

use audis_common::{AudisError, Result, UpdateChannel};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;

/// Where releases are published.
const RELEASES_API: &str = "https://api.github.com/repos/neura-spheres/Audis/releases?per_page=30";

/// Anything the updater is pointed at must live under this prefix.
const RELEASES_PREFIX: &str = "https://github.com/neura-spheres/Audis/releases/";

/// The updater manifest CI publishes alongside the installers.
const MANIFEST_ASSET: &str = "latest.json";

/// GitHub rejects requests without a user agent.
const USER_AGENT: &str = "Audis-Updater";

/// Give up rather than leave the user watching a spinner.
const TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

/// A release newer than the one running.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInfo {
    /// Semver without the leading `v`, e.g. `1.2.3` or `1.2.3-beta.1`.
    pub version: String,
    /// The git tag, e.g. `v1.2.3`.
    pub tag: String,
    /// Release notes, as written on the release.
    pub notes: String,
    /// The release page, for the user to download from.
    pub url: String,
    /// True for a beta.
    pub prerelease: bool,
    /// When it was published, as GitHub reports it.
    pub published_at: Option<String>,
    /// The updater manifest for this release, when one was published with it.
    ///
    /// `None` means the release predates the updater, or CI did not publish a
    /// manifest: the user can still download it by hand, but Audis cannot
    /// install it for them.
    pub manifest_url: Option<String>,
}

/// The result of looking for a new version.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateCheck {
    /// The version running now.
    pub current_version: String,
    /// The newer release, when there is one.
    pub update: Option<ReleaseInfo>,
    /// Which channel was consulted.
    pub channel: UpdateChannel,
}

/// One release, as the GitHub API returns it.
#[derive(Debug, Deserialize)]
struct GithubRelease {
    tag_name: String,
    #[serde(default)]
    body: Option<String>,
    html_url: String,
    #[serde(default)]
    prerelease: bool,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    published_at: Option<String>,
    #[serde(default)]
    assets: Vec<GithubAsset>,
}

/// One file attached to a release.
#[derive(Debug, Deserialize)]
struct GithubAsset {
    name: String,
    browser_download_url: String,
}

/// Look for a release newer than `current` on the given channel.
pub async fn check(channel: UpdateChannel, current: &str) -> Result<UpdateCheck> {
    let running = semver::Version::parse(current).map_err(|error| AudisError::Configuration {
        detail: format!("this build has a version Audis cannot read ({current}): {error}"),
    })?;

    let releases = fetch().await?;

    let newest = releases
        .into_iter()
        .filter(|release| !release.draft)
        .filter_map(|release| {
            let version = parse_tag(&release.tag_name)?;
            Some((version, release))
        })
        // A beta is offered only on the beta channel. GitHub's own flag and the
        // tag can disagree, so a release counts as a beta if either says so.
        .filter(|(version, release)| match channel {
            UpdateChannel::Beta => true,
            UpdateChannel::Stable => !release.prerelease && version.pre.is_empty(),
        })
        .max_by(|(left, _), (right, _)| left.cmp(right));

    let update = newest
        .filter(|(version, _)| *version > running)
        .map(|(version, release)| ReleaseInfo {
            version: version.to_string(),
            tag: release.tag_name,
            notes: release.body.unwrap_or_default().trim().to_owned(),
            url: release.html_url,
            prerelease: release.prerelease || !version.pre.is_empty(),
            published_at: release.published_at,
            manifest_url: release
                .assets
                .into_iter()
                .find(|asset| asset.name == MANIFEST_ASSET)
                .map(|asset| asset.browser_download_url)
                .filter(|url| url.starts_with(RELEASES_PREFIX)),
        });

    match &update {
        Some(release) => tracing::info!(
            %release.version, ?channel, "a newer version is available"
        ),
        None => tracing::info!(?channel, "already up to date"),
    }

    Ok(UpdateCheck {
        current_version: current.to_owned(),
        update,
        channel,
    })
}

/// Ask GitHub for the published releases.
async fn fetch() -> Result<Vec<GithubRelease>> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| AudisError::Configuration {
            detail: format!("the update check could not start: {error}"),
        })?;

    let response = client
        .get(RELEASES_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|error| AudisError::Configuration {
            detail: format!("Audis could not reach GitHub to check for updates: {error}"),
        })?;

    if !response.status().is_success() {
        return Err(AudisError::Configuration {
            detail: format!(
                "GitHub refused the update check with status {}.",
                response.status().as_u16()
            ),
        });
    }

    let body = response
        .text()
        .await
        .map_err(|error| AudisError::Configuration {
            detail: format!("the update check returned nothing readable: {error}"),
        })?;

    serde_json::from_str(&body).map_err(|error| AudisError::Configuration {
        detail: format!("the update check returned something unexpected: {error}"),
    })
}

/// Read `v1.2.3` or `1.2.3-beta.1` into a version, ignoring anything else.
fn parse_tag(tag: &str) -> Option<semver::Version> {
    semver::Version::parse(tag.trim().trim_start_matches(['v', 'V'])).ok()
}

/// Download and install a release, then restart into it.
///
/// The manifest is pointed at the release the channel chose, because GitHub has
/// no "latest pre-release" address a static endpoint could use. Whatever the
/// manifest names is still only installed if its signature matches the key built
/// into this binary, so a manifest that has been tampered with buys nothing.
///
/// This does not return: a successful install restarts the app.
pub async fn install(app: &AppHandle, manifest_url: &str) -> Result<()> {
    if !manifest_url.starts_with(RELEASES_PREFIX) {
        return Err(AudisError::InvalidArgument {
            field: "manifestUrl".to_owned(),
            detail: "that update manifest does not come from Audis".to_owned(),
        });
    }

    let endpoint =
        manifest_url
            .parse::<tauri::Url>()
            .map_err(|error| AudisError::Configuration {
                detail: format!("the update manifest address could not be read: {error}"),
            })?;

    let updater = app
        .updater_builder()
        .endpoints(vec![endpoint])
        .map_err(as_update_error)?
        .build()
        .map_err(as_update_error)?;

    let Some(update) = updater.check().await.map_err(as_update_error)? else {
        tracing::info!("the update disappeared between finding it and installing it");
        return Ok(());
    };

    tracing::info!(version = %update.version, "installing update");

    let mut downloaded: u64 = 0;

    update
        .download_and_install(
            |chunk, total| {
                downloaded += chunk as u64;
                app.emit(
                    audis_common::events::UPDATE_PROGRESS,
                    serde_json::json!({ "downloaded": downloaded, "total": total }),
                )
                .ok();
            },
            || {
                tracing::info!("update downloaded; handing over to the installer");
            },
        )
        .await
        .map_err(as_update_error)?;

    tracing::info!("update installed; restarting");
    app.restart();
}

/// Updater failures are all "something about this install went wrong".
fn as_update_error(error: tauri_plugin_updater::Error) -> AudisError {
    AudisError::Configuration {
        detail: format!("the update could not be installed: {error}"),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn tags_parse_with_or_without_the_v_prefix() {
        assert_eq!(
            parse_tag("v1.2.3").expect("a valid tag").to_string(),
            "1.2.3"
        );
        assert_eq!(
            parse_tag("1.2.3").expect("a valid tag").to_string(),
            "1.2.3"
        );
        assert_eq!(
            parse_tag("v1.2.3-beta.1").expect("a valid tag").to_string(),
            "1.2.3-beta.1"
        );
    }

    #[test]
    fn a_tag_that_is_not_a_version_is_ignored_rather_than_failing_the_check() {
        assert!(parse_tag("nightly").is_none());
        assert!(parse_tag("").is_none());
    }

    #[test]
    fn a_beta_sorts_before_the_release_it_leads_to() {
        // The point of the tag convention: a beta tester is not stranded on the
        // pre-release once the finished version ships.
        let beta = parse_tag("v1.2.0-beta.1").expect("a valid tag");
        let stable = parse_tag("v1.2.0").expect("a valid tag");
        assert!(beta < stable);
    }

    #[test]
    fn betas_order_among_themselves() {
        assert!(
            parse_tag("v1.2.0-beta.2").expect("a valid tag")
                > parse_tag("v1.2.0-beta.1").expect("a valid tag")
        );
    }

    #[test]
    fn a_beta_of_a_later_version_still_beats_the_current_release() {
        assert!(
            parse_tag("v1.3.0-beta.1").expect("a valid tag")
                > parse_tag("v1.2.0").expect("a valid tag")
        );
    }
}
