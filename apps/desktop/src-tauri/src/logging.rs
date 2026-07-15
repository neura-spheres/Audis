//! Structured logging.
//!
//! Logs must never contain API keys, authorization headers, provider payloads,
//! raw audio, voice profiles, or transcript text. The defence is layered: the
//! provider code never hands secrets to `tracing`, the default level sits below
//! where request bodies would be traced, and files stay under `%LOCALAPPDATA%`
//! rather than a synced folder.

use std::sync::OnceLock;

use audis_common::{AppPaths, identity::ENV_PREFIX};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Holds the non-blocking writer alive for the process lifetime. Dropping the
/// guard flushes buffered lines, and losing it would truncate the log at exit,
/// which is exactly when the interesting lines get written.
static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Initialise tracing. Call once, as early in startup as possible.
///
/// Writes a daily rolling file under `<data>/logs` and, in debug builds, to
/// stderr as well. `AUDIS_LOG` overrides the filter.
pub fn init(paths: &AppPaths) {
    let filter = EnvFilter::try_from_env(format!("{ENV_PREFIX}LOG"))
        .unwrap_or_else(|_| EnvFilter::new("info,wry=warn,tao=warn,hyper=warn"));

    let file_appender = tracing_appender::rolling::daily(
        paths.logs_dir(),
        format!("{}.log", audis_common::identity::LOG_PREFIX),
    );
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    // If this somehow runs twice the second guard drops and flushes, which is
    // harmless and better than panicking during startup.
    let _ = LOG_GUARD.set(guard);

    let file_layer = fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    // A release build is windowed and has no console attached, so stderr is
    // only useful during development.
    #[cfg(debug_assertions)]
    let registry = registry.with(fmt::layer().with_writer(std::io::stderr).with_target(false));

    registry.init();

    // Correlation id for this run, so an exported diagnostic bundle can be tied
    // to one launch without identifying the user.
    let launch_id = uuid::Uuid::new_v4();
    tracing::info!(
        launch_id = %launch_id,
        version = env!("CARGO_PKG_VERSION"),
        data_dir = %paths.root().display(),
        "Audis starting"
    );
}
