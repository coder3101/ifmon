mod app;
mod network;
mod types;
mod ui;
mod utils;

use std::time::Duration;

use crate::app::App;

/// Parse an update interval (in milliseconds) from an optional CLI argument or
/// the `IFMON_UPDATE_INTERVAL_MS` environment variable. Returns `None` to use the default.
fn parse_update_interval() -> Option<Duration> {
    let raw = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("IFMON_UPDATE_INTERVAL_MS").ok())?;

    raw.parse::<u64>()
        .ok()
        .map(|ms| Duration::from_millis(ms.max(50))) // enforce a sane lower bound
}

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();

    let app = match parse_update_interval() {
        Some(interval) => App::with_update_interval(interval),
        None => App::new(),
    };

    let result = app.run(terminal);
    ratatui::restore();
    result
}
