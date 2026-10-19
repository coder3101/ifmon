//! Interactive dashboard.

mod app;
mod export;
mod ui;

use std::io;
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;
use crossterm::event::{self, DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use ratatui::DefaultTerminal;

use crate::settings::Settings;

pub use app::App;

/// How often the interface list is re-enumerated to pick up hot-plugged adapters.
const REFRESH_EVERY: Duration = Duration::from_secs(5);

pub fn run(settings: Settings) -> Result<()> {
    let mouse = settings.mouse;
    let mut app = App::new(settings);

    let mut terminal = ratatui::init();
    if mouse {
        execute!(io::stdout(), EnableMouseCapture)?;
        // ratatui's panic hook restores the screen; also release the mouse.
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            let _ = execute!(io::stdout(), DisableMouseCapture);
            previous(info);
        }));
    }

    let result = event_loop(&mut terminal, &mut app);

    if mouse {
        let _ = execute!(io::stdout(), DisableMouseCapture);
    }
    ratatui::restore();
    result
}

fn event_loop(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    app.monitor.sample();
    let mut next_sample = Instant::now() + app.interval;
    let mut next_refresh = Instant::now() + REFRESH_EVERY;
    let mut dirty = true;

    while app.running {
        if dirty {
            terminal.draw(|frame| ui::draw(frame, app))?;
            dirty = false;
        }

        let timeout = next_sample.saturating_duration_since(Instant::now());
        if event::poll(timeout)? {
            // Drain bursts (e.g. mouse motion) before redrawing once.
            loop {
                app.handle_event(&event::read()?);
                if !app.running || !event::poll(Duration::ZERO)? {
                    break;
                }
            }
            dirty = true;
        }

        let now = Instant::now();
        if now >= next_sample {
            if now >= next_refresh {
                app.monitor.refresh_interfaces();
                next_refresh = now + REFRESH_EVERY;
            }
            app.monitor.sample();
            next_sample += app.interval;
            if next_sample < now {
                next_sample = now + app.interval;
            }
            dirty = true;
        }
        dirty |= app.expire_toast();
    }
    Ok(())
}
