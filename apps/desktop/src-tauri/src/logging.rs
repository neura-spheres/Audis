//! Structured logging.

use std::sync::OnceLock;

use audis_common::{AppPaths, identity::ENV_PREFIX};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Holds the non-blocking writer alive for the process lifetime. Dropping the
static LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();

/// Initialise tracing. Call once, as early in startup as possible.
pub fn init(paths: &AppPaths) {
    let filter = EnvFilter::try_from_env(format!("{ENV_PREFIX}LOG"))
        .unwrap_or_else(|_| EnvFilter::new("info,wry=warn,tao=warn,hyper=warn"));

    let file_appender = tracing_appender::rolling::daily(
        paths.logs_dir(),
        format!("{}.log", audis_common::identity::LOG_PREFIX),
    );
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    let _ = LOG_GUARD.set(guard);

    let file_layer = fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true);

    let registry = tracing_subscriber::registry().with(filter).with(file_layer);

    #[cfg(debug_assertions)]
    let registry = registry.with(fmt::layer().with_writer(std::io::stderr).with_target(false));

    registry.init();

    let launch_id = uuid::Uuid::new_v4();
    tracing::info!(
        launch_id = %launch_id,
        version = crate::APP_VERSION,
        data_dir = %paths.root().display(),
        "Audis starting"
    );
}
