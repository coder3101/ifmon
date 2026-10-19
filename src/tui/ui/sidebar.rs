use netdev::Interface;
use netdev::interface::state::OperState;
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{
    HighlightSpacing, List, ListItem, Scrollbar, ScrollbarOrientation, ScrollbarState,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use super::panel;
use crate::net::{Track, display_name};
use crate::theme::Theme;
use crate::tui::app::{App, SortKey};

/// Rows per interface entry (name line + rates line).
pub const ITEM_HEIGHT: usize = 2;
const HIGHLIGHT: &str = "\u{258C} ";
const SPARK: [char; 8] = [
    '\u{2581}', '\u{2582}', '\u{2583}', '\u{2584}', '\u{2585}', '\u{2586}', '\u{2587}', '\u{2588}',
];
const RATE_WIDTH: usize = 10;

/// Sidebar width: enough for the longest name and the rate columns, capped to leave room.
pub fn width(app: &App, total: u16) -> u16 {
    let longest = app
        .visible()
        .iter()
        .map(|i| display_name(i).width())
        .max()
        .unwrap_or(8);
    let wanted = (longest + 18).max(32) as u16;
    let cap = (total * 2 / 5).max(22);
    wanted.min(cap).min(total)
}

/// Unicode block sparkline of the last `width` combined rx+tx samples.
fn sparkline(track: Option<&Track>, width: usize) -> String {
    let Some(track) = track else {
        return String::new();
    };
    let values: Vec<f64> = track.history.recent(width).map(|p| p.rx + p.tx).collect();
    let max = values.iter().copied().fold(0.0, f64::max);
    if max <= 0.0 {
        return SPARK[0].to_string().repeat(values.len());
    }
    values
        .iter()
        .map(|v| SPARK[((v / max) * 7.0).round().clamp(0.0, 7.0) as usize])
        .collect()
}

fn truncate(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for ch in text.chars() {
        let w = ch.width().unwrap_or(1);
        if used + w + 1 > width {
            break;
        }
        out.push(ch);
        used += w;
    }
    out.push('\u{2026}');
    out
}

fn status_dot(iface: &Interface, theme: &Theme) -> Span<'static> {
    let color = match iface.oper_state {
        OperState::Up => theme.ok,
        OperState::Unknown if iface.is_running() => theme.ok,
        OperState::Unknown | OperState::Dormant | OperState::Testing => theme.warn,
        _ => theme.danger,
    };
    Span::styled("\u{25CF} ", Style::new().fg(color))
}

fn item<'a>(iface: &Interface, track: Option<&Track>, inner: usize, app: &App) -> ListItem<'a> {
    let theme = &app.theme;
    let star = if iface.default { " \u{2605}" } else { "" };
    let spark_width = inner
        .saturating_sub(display_name(iface).width() + star.width() + 3)
        .min(12);
    let name_width = inner.saturating_sub(2 + star.width() + spark_width + 1);
    let name = truncate(display_name(iface), name_width);
    let spark = sparkline(track, spark_width);
    let gap = inner.saturating_sub(2 + name.width() + star.width() + spark.width());

    let line1 = Line::from(vec![
        status_dot(iface, theme),
        Span::styled(name, Style::new().fg(theme.text).bold()),
        Span::styled(star, Style::new().fg(theme.highlight)),
        Span::raw(" ".repeat(gap)),
        Span::styled(spark, Style::new().fg(theme.accent)),
    ]);

    let (rx, tx) = track.map_or((0.0, 0.0), |t| (t.rate.rx_bytes, t.rate.tx_bytes));
    let line2 = Line::from(vec![
        Span::raw("  "),
        Span::styled(
            format!("\u{2193}{:>RATE_WIDTH$}", app.units.rate(rx)),
            Style::new().fg(theme.rx),
        ),
        Span::raw("  "),
        Span::styled(
            format!("\u{2191}{:>RATE_WIDTH$}", app.units.rate(tx)),
            Style::new().fg(theme.tx),
        ),
    ]);
    ListItem::new(Text::from(vec![line1, line2]))
}

/// Render the interface list; returns the inner list area for mouse hit-testing.
pub fn render(frame: &mut Frame, area: Rect, app: &mut App) -> Rect {
    let theme = app.theme.clone();
    let visible = app.visible();
    let total = app.data().interfaces().len();
    let inner_width = area.width.saturating_sub(2 + HIGHLIGHT.width() as u16) as usize;

    let items: Vec<ListItem> = visible
        .iter()
        .map(|iface| item(iface, app.data().track(&iface.name), inner_width, app))
        .collect();
    let count = items.len();

    let scope = if app.show_all { "All" } else { "Physical" };
    let title = Line::from(vec![
        Span::raw(format!(" {scope} ")),
        Span::styled(format!("{count}/{total} "), Style::new().fg(theme.dim)),
    ]);
    let mut block = panel(&theme, title, true);
    if app.sort != SortKey::System {
        block = block.title_bottom(
            Line::from(Span::styled(
                format!(" sort: {} ", app.sort.label()),
                Style::new().fg(theme.dim),
            ))
            .right_aligned(),
        );
    }
    let inner = block.inner(area);

    let list = List::new(items)
        .block(block)
        .highlight_style(theme.selection())
        .highlight_symbol(Line::from(Span::styled(
            HIGHLIGHT,
            Style::new().fg(theme.accent),
        )))
        .highlight_spacing(HighlightSpacing::Always);

    let mut state = std::mem::take(&mut app.list_state);
    frame.render_stateful_widget(list, area, &mut state);

    let rows_visible = usize::from(inner.height) / ITEM_HEIGHT;
    if count > rows_visible && inner.height > 0 {
        let mut sb = ScrollbarState::new(count.saturating_sub(rows_visible) + 1)
            .viewport_content_length(rows_visible)
            .position(state.offset());
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .style(Style::new().fg(theme.border)),
            area,
            &mut sb,
        );
    }
    app.list_state = state;
    inner
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_by_display_width() {
        assert_eq!(truncate("en0", 10), "en0");
        assert_eq!(truncate("verylongname", 5), "very\u{2026}");
        assert_eq!(truncate("ab", 0), "\u{2026}");
    }
}
