use ratatui::Frame;
use ratatui::layout::{Constraint, Rect};
use ratatui::style::{Color, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Cell, Row, Table};

use super::panel;
use crate::net::{Summary, Track};
use crate::theme::Theme;
use crate::tui::app::App;
use crate::units::{self, short_duration};

fn stat_row<'a>(theme: &Theme, name: &'a str, rx: String, tx: String) -> Row<'a> {
    stat_row_styled(theme, name, (rx, theme.rx), (tx, theme.tx))
}

fn stat_row_styled<'a>(
    theme: &Theme,
    name: &'a str,
    rx: (String, Color),
    tx: (String, Color),
) -> Row<'a> {
    Row::new(vec![
        Cell::from(Span::styled(name, Style::new().fg(theme.dim))),
        Cell::from(Line::from(Span::styled(rx.0, Style::new().fg(rx.1))).right_aligned()),
        Cell::from(Line::from(Span::styled(tx.0, Style::new().fg(tx.1))).right_aligned()),
    ])
}

fn utilization(rate_bytes: f64, link_bits: Option<u64>) -> Option<f64> {
    let link = link_bits.filter(|&l| l > 0)? as f64;
    Some((rate_bytes * 8.0 / link * 100.0).min(100.0))
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let units = app.units;
    let block = panel(
        theme,
        Line::from(vec![
            Span::raw(" Statistics "),
            Span::styled(
                format!("\u{b7} last {} ", short_duration(app.window)),
                Style::new().fg(theme.dim),
            ),
        ]),
        false,
    );

    let Some(track) = app.selected_track() else {
        frame.render_widget(
            ratatui::widgets::Paragraph::new("Waiting for the first sample\u{2026}")
                .style(Style::new().fg(theme.dim))
                .block(block),
            area,
        );
        return;
    };

    let data = app.data();
    let since = data.now() - app.window.as_secs_f64();
    let window: Vec<_> = track.history.since(since).collect();
    let rx = Summary::from_values(window.iter().map(|p| p.rx));
    let tx = Summary::from_values(window.iter().map(|p| p.tx));
    let rate = |v: f64| units.rate(v);

    let header = Row::new(vec![
        Cell::from(""),
        Cell::from(Line::from("\u{2193} Download".fg(theme.rx).bold()).right_aligned()),
        Cell::from(Line::from("\u{2191} Upload".fg(theme.tx).bold()).right_aligned()),
    ]);

    let mut rows = vec![
        stat_row_styled(
            theme,
            "Current",
            (rate(track.rate.rx_bytes), theme.rx),
            (rate(track.rate.tx_bytes), theme.tx),
        )
        .bold(),
        stat_row(theme, "Average", rate(rx.avg), rate(tx.avg)),
        stat_row(theme, "Peak", rate(rx.max), rate(tx.max)),
        stat_row(theme, "95th pct", rate(rx.p95), rate(tx.p95)),
        stat_row(theme, "Minimum", rate(rx.min), rate(tx.min)),
        stat_row(
            theme,
            "Session peak",
            rate(track.peak_rx),
            rate(track.peak_tx),
        ),
        stat_row(
            theme,
            "Session total",
            units.size(track.session.rx_bytes),
            units.size(track.session.tx_bytes),
        ),
        stat_row(
            theme,
            "Since boot",
            units.size(track.raw.rx_bytes),
            units.size(track.raw.tx_bytes),
        ),
    ];

    if data.extended() {
        rows.push(stat_row(
            theme,
            "Packets/s",
            units::count(track.rate.rx_packets),
            units::count(track.rate.tx_packets),
        ));
        let bad = |v: u64| {
            let color = if v > 0 { theme.danger } else { theme.dim };
            (units::count(v as f64), color)
        };
        rows.push(stat_row_styled(
            theme,
            "Errors",
            bad(track.session.rx_errors),
            bad(track.session.tx_errors),
        ));
        rows.push(stat_row_styled(
            theme,
            "Drops",
            bad(track.session.rx_drops),
            bad(track.session.tx_drops),
        ));
    }

    if let Some(iface) = app.selected_interface() {
        let rx_util = utilization(track.rate.rx_bytes, iface.receive_speed);
        let tx_util = utilization(track.rate.tx_bytes, iface.transmit_speed);
        if rx_util.is_some() || tx_util.is_some() {
            let fmt = |u: Option<f64>| {
                u.map_or_else(
                    || ("-".to_string(), theme.dim),
                    |u| {
                        let color = match u {
                            u if u >= 90.0 => theme.danger,
                            u if u >= 70.0 => theme.warn,
                            _ => theme.ok,
                        };
                        (format!("{u:.1}%"), color)
                    },
                )
            };
            rows.push(stat_row_styled(
                theme,
                "Link usage",
                fmt(rx_util),
                fmt(tx_util),
            ));
        }
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(13),
            Constraint::Fill(1),
            Constraint::Fill(1),
        ],
    )
    .header(header)
    .column_spacing(2)
    .block(block);
    frame.render_widget(table, area);
}

/// Window summary for one direction, used by the graph titles.
pub fn window_summary(track: &Track, since: f64, rx: bool) -> Summary {
    Summary::from_values(
        track
            .history
            .since(since)
            .map(|p| if rx { p.rx } else { p.tx }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utilization_is_percent_of_link() {
        assert_eq!(utilization(12_500_000.0, Some(1_000_000_000)), Some(10.0));
        assert_eq!(utilization(1e12, Some(1_000)), Some(100.0));
        assert_eq!(utilization(1.0, Some(0)), None);
        assert_eq!(utilization(1.0, None), None);
    }
}
