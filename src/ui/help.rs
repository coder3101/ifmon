use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

/// Render the help screen
pub fn render_help(frame: &mut Frame) {
    let help_text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Network Interface Monitor", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Keyboard Shortcuts:", Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab          ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("- Switch to next interface"),
        ]),
        Line::from(vec![
            Span::styled("  Shift+Tab    ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("- Switch to previous interface"),
        ]),
        Line::from(vec![
            Span::styled("  h            ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("- Toggle this help screen"),
        ]),
        Line::from(vec![
            Span::styled("  f            ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("- Toggle show all/physical interfaces"),
        ]),
        Line::from(vec![
            Span::styled("  Up/Down      ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("- Scroll IP addresses"),
        ]),
        Line::from(vec![
            Span::styled("  q / Esc      ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::raw("- Quit application"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Features:", Style::default().fg(Color::Yellow).add_modifier(Modifier::UNDERLINED)),
        ]),
        Line::from(""),
        Line::from("  • Real-time network speed monitoring"),
        Line::from("  • RX/TX speed graphs and statistics"),
        Line::from("  • Historical data visualization"),
        Line::from("  • Peak speed tracking"),
        Line::from("  • Total bytes transferred"),
        Line::from("  • Interface filtering"),
        Line::from(""),
        Line::from(vec![
            Span::styled("Press ", Style::default().fg(Color::DarkGray)),
            Span::styled("h", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" or ", Style::default().fg(Color::DarkGray)),
            Span::styled("Esc", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
            Span::styled(" to close this help", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    
    let help_paragraph = Paragraph::new(help_text)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help ")
        )
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true });
    
    let area = frame.area();
    let vertical = Layout::vertical([
        Constraint::Percentage(10),
        Constraint::Min(20),
        Constraint::Percentage(10),
    ]).split(area);
    let horizontal = Layout::horizontal([
        Constraint::Percentage(20),
        Constraint::Min(60),
        Constraint::Percentage(20),
    ]).split(vertical[1]);
    
    frame.render_widget(help_paragraph, horizontal[1]);
}
