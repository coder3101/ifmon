//! Header and footer status lines.

use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use super::key_hint;
use crate::net::is_relevant;
use crate::tui::app::{App, Mode, ToastKind};
use crate::units::{clock, short_duration};

pub fn render_header(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let base = Style::new().bg(theme.bar_bg);
    frame.render_widget(Paragraph::new("").style(base), area);

    let mut left = vec![
        Span::styled(" ifmon ", theme.badge(theme.accent)),
        Span::styled(
            format!(" v{} ", env!("CARGO_PKG_VERSION")),
            Style::new().fg(theme.dim),
        ),
    ];
    if app.paused() {
        left.push(Span::styled(" \u{23F8} PAUSED ", theme.badge(theme.warn)));
    }

    // Aggregate throughput across the interfaces in the current filter mode.
    let data = app.data();
    let (rx, tx) = data
        .interfaces()
        .iter()
        .filter(|i| app.show_all || is_relevant(i))
        .filter_map(|i| data.track(&i.name))
        .fold((0.0, 0.0), |(rx, tx), t| {
            (rx + t.rate.rx_bytes, tx + t.rate.tx_bytes)
        });
    left.extend([
        Span::styled("  \u{3A3} ", Style::new().fg(theme.dim)),
        Span::styled(
            format!("\u{2193} {}", app.units.rate(rx)),
            Style::new().fg(theme.rx),
        ),
        Span::raw("  "),
        Span::styled(
            format!("\u{2191} {}", app.units.rate(tx)),
            Style::new().fg(theme.tx),
        ),
    ]);

    let right = Line::from(vec![
        Span::styled("\u{23F1} ", Style::new().fg(theme.dim)),
        Span::styled(clock(data.uptime()), Style::new().fg(theme.text)),
        Span::styled("  every ", Style::new().fg(theme.dim)),
        Span::styled(short_duration(app.interval), Style::new().fg(theme.text)),
        Span::styled("  window ", Style::new().fg(theme.dim)),
        Span::styled(short_duration(app.window), Style::new().fg(theme.text)),
        Span::styled("  ", Style::new()),
        Span::styled(app.units.label(), Style::new().fg(theme.text)),
        Span::raw(" "),
    ])
    .right_aligned();

    let left = Line::from(left);
    let left_width = left.width() as u16;
    frame.render_widget(Paragraph::new(left).style(base), area);
    if area.width >= left_width + right.width() as u16 + 2 {
        frame.render_widget(Paragraph::new(right).style(base), area);
    }
}

pub fn render_footer(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let base = Style::new().bg(theme.bar_bg);

    let line = if app.mode == Mode::Filter {
        Line::from(vec![
            Span::styled(" / ", theme.badge(theme.highlight)),
            Span::styled(format!(" {}", app.filter), Style::new().fg(theme.text)),
            Span::styled("\u{258F}", Style::new().fg(theme.highlight)),
            Span::styled(
                "   Enter apply \u{b7} Esc clear \u{b7} \u{2191}\u{2193} select",
                Style::new().fg(theme.dim),
            ),
        ])
    } else if let Some(toast) = &app.toast {
        let color = match toast.kind {
            ToastKind::Info => theme.accent,
            ToastKind::Error => theme.danger,
        };
        Line::from(vec![
            Span::styled(" \u{25CF} ", Style::new().fg(color)),
            Span::styled(toast.text.clone(), Style::new().fg(theme.text)),
        ])
    } else {
        let mut spans = vec![Span::raw(" ")];
        if !app.filter.is_empty() {
            spans.push(Span::styled(
                format!("filter: {} ", app.filter),
                Style::new().fg(theme.highlight),
            ));
            spans.push(Span::raw(" "));
        }
        let hints: &[(&str, &str)] = &[
            ("q", "quit"),
            ("\u{2191}\u{2193}", "select"),
            ("/", "filter"),
            ("s", "sort"),
            ("v", "view"),
            ("m", "style"),
            ("+/-", "zoom"),
            ("b", "bits"),
            ("space", "pause"),
            ("e", "export"),
            ("t", "theme"),
            ("?", "help"),
        ];
        for (k, d) in hints {
            spans.extend(key_hint(theme, k, d));
        }
        Line::from(spans)
    };
    frame.render_widget(Paragraph::new(line).style(base), area);
}
