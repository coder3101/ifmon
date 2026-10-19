use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Clear, Paragraph};

use super::panel;
use crate::theme::Theme;
use crate::tui::app::App;

const NAVIGATION: &[(&str, &str)] = &[
    ("\u{2191}\u{2193} j k", "Select interface"),
    ("g / G", "First / last interface"),
    ("1-9", "Jump to interface"),
    ("/", "Filter by name"),
    ("s", "Cycle sort order"),
    ("f / a", "Show all interfaces"),
    ("J / K", "Scroll details"),
    ("Mouse", "Click, scroll, zoom"),
];

const DISPLAY: &[(&str, &str)] = &[
    ("v", "Cycle graph view"),
    ("m", "Cycle graph style"),
    ("+ / -", "Zoom time window"),
    ("b", "Toggle bits / bytes"),
    ("t", "Cycle color theme"),
    ("i", "Toggle details panel"),
];

const ACTIONS: &[(&str, &str)] = &[
    ("Space / p", "Pause / resume"),
    ("r", "Reset session stats"),
    ("e", "Export CSV"),
    ("? / h", "Toggle this help"),
    ("q / Esc", "Quit"),
];

fn section<'a>(theme: &Theme, heading: &'a str, keys: &[(&'a str, &'a str)]) -> Vec<Line<'a>> {
    let mut lines = vec![Line::from(Span::styled(
        heading,
        Style::new().fg(theme.accent).bold(),
    ))];
    lines.extend(keys.iter().map(|(k, d)| {
        Line::from(vec![
            Span::styled(
                format!("  {k:<12}"),
                Style::new().fg(theme.highlight).bold(),
            ),
            Span::styled(*d, Style::new().fg(theme.text)),
        ])
    }));
    lines.push(Line::from(""));
    lines
}

/// Popup width needed to show the two sections side by side.
const TWO_COLUMN_WIDTH: u16 = 80;

pub fn render(frame: &mut Frame, app: &App) {
    let theme = &app.theme;

    let mut left = section(theme, "Navigation", NAVIGATION);
    let config = app.config_path.as_ref().map_or_else(
        || "none (run `ifmon config init`)".to_string(),
        |p| p.display().to_string(),
    );
    left.push(Line::from(vec![
        Span::styled("Config  ", Style::new().fg(theme.dim)),
        Span::styled(config, Style::new().fg(theme.text)),
    ]));
    let mut right = section(theme, "Display", DISPLAY);
    right.extend(section(theme, "Actions", ACTIONS));

    let two_columns = frame.area().width >= TWO_COLUMN_WIDTH + 2;
    let rows = if two_columns {
        left.len().max(right.len())
    } else {
        left.len() + right.len()
    };
    let width = if two_columns { TWO_COLUMN_WIDTH } else { 44 };
    let area = frame
        .area()
        .centered(Constraint::Max(width), Constraint::Max(rows as u16 + 2));
    frame.render_widget(Clear, area);

    let block = panel(theme, " Keyboard shortcuts ", true).title_bottom(
        Line::from(Span::styled(" Esc to close ", Style::new().fg(theme.dim))).right_aligned(),
    );
    let inner = block.inner(area).inner(Margin::new(1, 0));
    frame.render_widget(block, area);

    if two_columns {
        let [l, r] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(inner);
        frame.render_widget(Paragraph::new(left), l);
        frame.render_widget(Paragraph::new(right), r);
    } else {
        left.extend(right);
        frame.render_widget(Paragraph::new(left), inner);
    }
}
