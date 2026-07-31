use std::collections::HashMap;

use netdev::Interface;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, HighlightSpacing, List, ListItem, ListState},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::ui::theme::Theme;
use crate::utils::format_bytes;

/// Truncate `text` (by display width) with an ellipsis if it exceeds `width`.
fn truncate_width(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let cap = width.saturating_sub(1); // reserve one column for the ellipsis
    let mut out = String::new();
    for ch in text.chars() {
        if out.width() + ch.width().unwrap_or(1) > cap {
            break;
        }
        out.push(ch);
    }
    format!("{out}\u{2026}")
}

/// Pad `text` with spaces up to `width` columns (display width aware).
fn pad_width(text: &str, width: usize) -> String {
    let cur = text.width();
    if cur >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - cur))
    }
}

/// Render the left-hand interface list with current RX/TX speeds.
pub fn render_sidebar(
    frame: &mut Frame,
    area: Rect,
    interfaces: &[Interface],
    selected: usize,
    speeds: &HashMap<u32, (f64, f64)>,
    show_all: bool,
    theme: &Theme,
) {
    let inner_width = area.width.saturating_sub(2) as usize;

    let items: Vec<ListItem> = interfaces
        .iter()
        .map(|iface| {
            let (rx, tx) = speeds.get(&iface.index).copied().unwrap_or((0.0, 0.0));
            let speed = format!("↓{} ↑{}", format_bytes(rx), format_bytes(tx));
            let name_width = inner_width.saturating_sub(speed.width()).saturating_sub(2);

            let name = truncate_width(&iface.name, name_width);

            ListItem::new(Line::from(vec![
                Span::raw(" "),
                Span::raw(pad_width(&name, name_width)),
                Span::raw(" "),
                Span::styled(format!("↓{}", format_bytes(rx)), theme.rx),
                Span::raw(" "),
                Span::styled(format!("↑{}", format_bytes(tx)), theme.tx),
            ]))
        })
        .collect();

    let title = if show_all {
        " All Interfaces "
    } else {
        " Physical "
    };
    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.border))
                .title(format!(" {title}({}) ", interfaces.len()))
                .title_style(
                    Style::default()
                        .fg(theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(
            Style::default()
                .fg(theme.highlight_fg)
                .bg(theme.selected_bg)
                .bold(),
        )
        .highlight_symbol("\u{25B8} ")
        .highlight_spacing(HighlightSpacing::Always);

    let mut state = ListState::default().with_selected(Some(selected));
    frame.render_stateful_widget(list, area, &mut state);
}
