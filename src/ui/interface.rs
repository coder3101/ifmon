use netdev::Interface;
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Style},
    text::Span,
    widgets::{Block, Borders, Row, Table},
    Frame,
};

use crate::utils::format_optional;

/// How many IPv4/IPv6 rows can be displayed at once in the fixed-height panel.
const IPV4_ROWS: usize = 2;
const IPV6_ROWS: usize = 1;

fn header_span(text: &str) -> Span<'_> {
    Span::styled(text, Style::default().fg(Color::Yellow))
}

/// Build rows for one address family, applying a family-specific scroll offset.
fn address_rows<'a, T: std::fmt::Display>(
    label: &'a str,
    addrs: &[T],
    scroll_offset: usize,
    max_rows: usize,
) -> Vec<Row<'a>> {
    if addrs.is_empty() {
        return vec![Row::new(vec![header_span(label), Span::raw("-")])];
    }

    let start = scroll_offset.min(addrs.len().saturating_sub(max_rows));

    let mut rows = Vec::new();
    let mut shown = 0;
    for (i, ip) in addrs.iter().skip(start).take(max_rows).enumerate() {
        rows.push(Row::new(vec![
            header_span(if i == 0 { label } else { "" }),
            Span::raw(ip.to_string()),
        ]));
        shown += 1;
    }

    let remaining = addrs.len() - start - shown;
    if remaining > 0 {
        rows.push(Row::new(vec![
            Span::raw(""),
            Span::styled(
                format!("(+{remaining} more, use \u{2191}\u{2193})"),
                Style::default().fg(Color::DarkGray),
            ),
        ]));
    }

    rows
}

/// Render the interface information panel.
pub fn render_interface_info(
    frame: &mut Frame,
    area: Rect,
    interface: &Interface,
    ipv4_scroll_offset: usize,
    ipv6_scroll_offset: usize,
) {
    let mut rows: Vec<Row> = vec![
        Row::new(vec![header_span("Name"), Span::raw(interface.name.clone())]),
        Row::new(vec![
            header_span("Type"),
            Span::raw(interface.if_type.name()),
        ]),
        Row::new(vec![
            header_span("MAC"),
            Span::raw(format_optional(&interface.mac_addr.map(|m| m.to_string()))),
        ]),
        Row::new(vec![
            header_span("MTU"),
            Span::raw(format_optional(&interface.mtu)),
        ]),
        Row::new(vec![
            header_span("State"),
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

    rows.extend(address_rows(
        "IPv4",
        &interface.ipv4,
        ipv4_scroll_offset,
        IPV4_ROWS,
    ));
    rows.extend(address_rows(
        "IPv6",
        &interface.ipv6,
        ipv6_scroll_offset,
        IPV6_ROWS,
    ));

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
