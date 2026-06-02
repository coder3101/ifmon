use netdev::Interface;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::utils::format_optional;

/// Render the interface information panel
pub fn render_interface_info(
    frame: &mut Frame,
    area: Rect,
    interface: &Interface,
    ip_scroll_offset: usize,
) {
    // Build rows with basic info
    let mut rows: Vec<Row> = vec![
        Row::new(vec![
            Span::styled("Name", Style::default().fg(Color::Yellow)),
            Span::raw(interface.name.clone()),
        ]),
        Row::new(vec![
            Span::styled("Type", Style::default().fg(Color::Yellow)),
            Span::raw(interface.if_type.name()),
        ]),
        Row::new(vec![
            Span::styled("MAC", Style::default().fg(Color::Yellow)),
            Span::raw(format_optional(&interface.mac_addr.map(|m| m.to_string()))),
        ]),
        Row::new(vec![
            Span::styled("MTU", Style::default().fg(Color::Yellow)),
            Span::raw(format_optional(&interface.mtu)),
        ]),
        Row::new(vec![
            Span::styled("State", Style::default().fg(Color::Yellow)),
            Span::styled(
                format!("{:?}", interface.oper_state),
                Style::default().fg(if format!("{:?}", interface.oper_state).contains("Up") {
                    Color::Green
                } else {
                    Color::Red
                }),
            ),
        ]),
    ];

    // Add IPv4 addresses (each on its own line)
    if interface.ipv4.is_empty() {
        rows.push(Row::new(vec![
            Span::styled("IPv4", Style::default().fg(Color::Yellow)),
            Span::raw("-"),
        ]));
    } else {
        for (i, ip) in interface
            .ipv4
            .iter()
            .skip(ip_scroll_offset)
            .take(2)
            .enumerate()
        {
            rows.push(Row::new(vec![
                Span::styled(
                    if i == 0 { "IPv4" } else { "" },
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(ip.to_string()),
            ]));
        }
        if interface.ipv4.len() > 2 {
            rows.push(Row::new(vec![
                Span::raw(""),
                Span::styled(
                    format!("(+{} more)", interface.ipv4.len() - 2),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    // Add IPv6 addresses (each on its own line)
    if interface.ipv6.is_empty() {
        rows.push(Row::new(vec![
            Span::styled("IPv6", Style::default().fg(Color::Yellow)),
            Span::raw("-"),
        ]));
    } else {
        for (i, ip) in interface
            .ipv6
            .iter()
            .skip(ip_scroll_offset)
            .take(1)
            .enumerate()
        {
            rows.push(Row::new(vec![
                Span::styled(
                    if i == 0 { "IPv6" } else { "" },
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw(ip.to_string()),
            ]));
        }
        if interface.ipv6.len() > 1 {
            rows.push(Row::new(vec![
                Span::raw(""),
                Span::styled(
                    format!("(+{} more, use ↑↓)", interface.ipv6.len() - 1),
                    Style::default().fg(Color::DarkGray),
                ),
            ]));
        }
    }

    let content = Table::new(rows, [Constraint::Length(12), Constraint::Fill(1)])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Interface Info ")
                .title_style(Style::default().fg(Color::Cyan)),
        )
        .column_spacing(2);

    frame.render_widget(content, area);
}
