mod bars;
mod details;
mod graphs;
mod help;
mod sidebar;
mod stats;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Wrap};

use super::app::{App, Mode};
use crate::theme::Theme;

pub use sidebar::ITEM_HEIGHT as SIDEBAR_ITEM_HEIGHT;

/// Below this width the details and statistics panels share one slot.
const WIDE_LAYOUT_MIN_WIDTH: u16 = 96;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let has_selection = app.sync_selection().is_some();

    let [header, body, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    bars::render_header(frame, header, app);
    bars::render_footer(frame, footer, app);

    let sidebar_width = sidebar::width(app, body.width);
    let [side, main] =
        Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Fill(1)]).areas(body);

    let list_area = sidebar::render(frame, side, app);
    app.areas.sidebar = side;
    app.areas.sidebar_list = list_area;

    if has_selection {
        render_main(frame, main, app);
    } else {
        render_empty(frame, main, app);
        app.areas.details = Rect::default();
        app.areas.graphs = Rect::default();
    }

    if app.mode == Mode::Help {
        help::render(frame, app);
    }
}

fn render_main(frame: &mut Frame, area: Rect, app: &mut App) {
    let top_height = match area.height {
        h if h >= 34 => 15,
        h if h >= 24 => 11,
        _ => 0,
    };
    let [top, graphs] =
        Layout::vertical([Constraint::Length(top_height), Constraint::Fill(1)]).areas(area);

    app.areas.details = Rect::default();
    if top_height > 0 {
        if area.width >= WIDE_LAYOUT_MIN_WIDTH && app.show_details {
            let [left, right] =
                Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
                    .areas(top);
            details::render(frame, left, app);
            stats::render(frame, right, app);
            app.areas.details = left;
        } else if app.show_details {
            details::render(frame, top, app);
            app.areas.details = top;
        } else {
            stats::render(frame, top, app);
        }
    }
    graphs::render(frame, graphs, app);
    app.areas.graphs = graphs;
}

fn render_empty(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let message = if app.filter.is_empty() {
        "No network interfaces to display.\n\nPress 'f' to include loopback, tunnel and virtual interfaces."
    } else {
        "No interfaces match the current filter.\n\nPress Esc to clear it."
    };
    let paragraph = Paragraph::new(message)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .style(Style::new().fg(theme.dim))
        .block(panel(theme, " Nothing here ", false));
    let inner = area.centered_vertically(Constraint::Length(6));
    frame.render_widget(paragraph, inner);
}

/// Standard rounded panel with a title.
pub(super) fn panel<'a>(theme: &Theme, title: impl Into<Line<'a>>, focused: bool) -> Block<'a> {
    let border = if focused {
        theme.border_focus
    } else {
        theme.border
    };
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(border))
        .title(
            title
                .into()
                .style(Style::new().fg(theme.accent).add_modifier(Modifier::BOLD)),
        )
}

/// Label/value styling shared by the info-style panels.
pub(super) fn label<'a>(theme: &Theme, text: &'a str) -> Span<'a> {
    Span::styled(text, Style::new().fg(theme.dim))
}

pub(super) fn key_hint<'a>(theme: &Theme, key: &'a str, desc: &'a str) -> [Span<'a>; 3] {
    [
        Span::styled(key, Style::new().fg(theme.highlight).bold()),
        Span::styled(format!(" {desc}"), Style::new().fg(theme.dim)),
        Span::raw("  "),
    ]
}

#[cfg(test)]
mod tests {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    use super::*;
    use crate::tui::app::tests::test_app;

    fn render(app: &mut App, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| draw(f, app)).unwrap();
        let buffer = terminal.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..h {
            for x in 0..w {
                out.push_str(buffer[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn renders_at_many_sizes_without_panicking() {
        for (w, h) in [(20, 5), (40, 12), (80, 24), (100, 30), (160, 50), (250, 70)] {
            let mut app = test_app();
            render(&mut app, w, h);
            app.mode = Mode::Help;
            render(&mut app, w, h);
        }
    }

    #[test]
    fn dashboard_shows_key_content() {
        let mut app = test_app();
        let screen = render(&mut app, 140, 40);
        assert!(screen.contains("ifmon"), "{screen}");
        assert!(screen.contains("en1"));
        assert!(screen.contains("Statistics"));
        assert!(screen.contains("Download"));
        assert!(screen.contains("Upload"));
        assert!(!screen.contains("lo0"), "loopback hidden by default");
    }

    #[test]
    fn every_view_and_style_renders() {
        let mut app = test_app();
        for _ in 0..3 {
            for _ in 0..4 {
                render(&mut app, 120, 40);
                app.style = app.style.next();
            }
            app.view = app.view.next();
        }
    }

    #[test]
    fn empty_filter_shows_message() {
        let mut app = test_app();
        app.filter = "zzz".into();
        let screen = render(&mut app, 100, 30);
        assert!(screen.contains("No interfaces match"));
    }

    #[test]
    fn records_hit_areas() {
        let mut app = test_app();
        render(&mut app, 140, 40);
        assert!(app.areas.sidebar.width > 0);
        assert!(app.areas.sidebar_list.height > 0);
        assert!(app.areas.graphs.height > 0);
        assert!(app.areas.details.width > 0);
    }
}
