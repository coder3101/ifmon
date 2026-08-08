use ratatui::{
    layout::{Alignment, Constraint, Layout},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

use super::theme::THEME;

fn key(label: &str) -> Span<'static> {
    Span::styled(
        format!("  {label:<12}"),
        Style::default().fg(THEME.tx).add_modifier(Modifier::BOLD),
    )
}

fn keyrow(label: &str, desc: &str) -> Line<'static> {
    Line::from(vec![key(label), Span::raw(format!("- {desc}"))])
}

fn heading(text: &str) -> Line<'static> {
    Line::from(vec![Span::styled(
        text.to_string(),
        Style::default()
            .fg(THEME.highlight_fg)
            .add_modifier(Modifier::UNDERLINED),
    )])
}

/// Render the help screen.
pub fn render_help(frame: &mut Frame) {
    let title = Line::from(vec![Span::styled(
        "Network Interface Monitor",
        Style::default()
            .fg(THEME.accent)
            .add_modifier(Modifier::BOLD),
    )]);

    let lines = vec![
        Line::from(""),
        title,
        Line::from(""),
        heading("Keyboard Shortcuts:"),
        Line::from(""),
        keyrow("Tab / j", "Switch to next interface"),
        keyrow("Shift+Tab / k", "Switch to previous interface"),
        keyrow("1-9", "Jump to interface by number"),
        keyrow("Click", "Click an interface in the sidebar"),
        keyrow("Up/Down", "Scroll IP addresses (or mouse wheel)"),
        keyrow("f", "Toggle all / physical interfaces"),
        keyrow("h", "Toggle this help screen"),
        keyrow("q / Esc", "Quit application"),
        Line::from(""),
        heading("Features:"),
        Line::from(""),
        Line::from("  • Real-time network speed monitoring"),
        Line::from("  • RX/TX speed graphs and statistics"),
        Line::from("  • Historical data visualization"),
        Line::from("  • Peak speed tracking (windowed)"),
        Line::from("  • Total bytes transferred"),
        Line::from("  • Interface filtering"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(THEME.dim)),
            Span::styled(
                "h",
                Style::default().fg(THEME.tx).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" or ", Style::default().fg(THEME.dim)),
            Span::styled(
                "Esc",
                Style::default().fg(THEME.tx).add_modifier(Modifier::BOLD),
            ),
            Span::styled(" to close this help", Style::default().fg(THEME.dim)),
        ]),
    ];

    let help_paragraph = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(THEME.border))
                .title(" Help ")
                .title_style(Style::default().fg(THEME.accent)),
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });

    let area = frame.area();
    let vertical = Layout::vertical([
        Constraint::Percentage(10),
        Constraint::Min(20),
        Constraint::Percentage(10),
    ])
    .split(area);
    let horizontal = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Min(60),
        Constraint::Percentage(20),
    ])
    .split(vertical[1]);

    frame.render_widget(help_paragraph, horizontal[1]);
}
