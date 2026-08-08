use std::collections::VecDeque;

use ratatui::{
    layout::{Alignment, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::Span,
    widgets::{Axis, Block, Borders, Chart, Dataset, GraphType, Paragraph},
    Frame,
};

use crate::types::SpeedHistory;
use crate::utils::{format_bytes, format_total_bytes};

use super::theme::THEME;

/// Data needed to render one speed history graph.
struct GraphData<'a> {
    timestamps: &'a VecDeque<f64>,
    speeds: &'a VecDeque<f64>,
    max_speed: f64,
    current_speed: f64,
    total_bytes: u64,
}

/// Render a single speed history graph. Used for both RX and TX.
fn render_speed_graph(
    frame: &mut Frame,
    area: Rect,
    data: GraphData,
    title: &str,
    color: Color,
    arrow: &str,
) {
    if data.speeds.is_empty() {
        let placeholder = Paragraph::new("Collecting data...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(THEME.border))
                    .title(format!(" {title} "))
                    .title_style(Style::default().fg(color)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, area);
        return;
    }

    // Dynamic scaling - use the peak within the current window.
    let max_speed = data.max_speed.max(1.0);

    let min_time = data.timestamps.front().copied().unwrap_or(0.0);
    let max_time = data
        .timestamps
        .back()
        .copied()
        .unwrap_or(100.0)
        .max(min_time + 1.0); // Ensure well-formed axis bounds.
    let span = (max_time - min_time).max(1.0);

    let data_points: Vec<(f64, f64)> = data
        .timestamps
        .iter()
        .zip(data.speeds.iter())
        .map(|(t, s)| (*t, *s))
        .collect();

    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(color))
        .data(&data_points)];

    // Time-relative (ago) X-axis labels: oldest on the left to "now" on the right.
    let x_labels = vec![
        Span::raw(format!("-{span:.0}s")),
        Span::raw(format!("-{:.0}s", span / 2.0)),
        Span::raw("0s"),
    ];

    let y_labels = vec![
        Span::raw("0"),
        Span::raw(format_bytes(max_speed / 2.0)),
        Span::raw(format_bytes(max_speed)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(THEME.border))
                .title(format!(
                    " {title} - Current: {} | Peak: {} | Total: {} ",
                    format_bytes(data.current_speed),
                    format_bytes(data.max_speed),
                    format_total_bytes(data.total_bytes)
                ))
                .title_style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        )
        .x_axis(
            Axis::default()
                .title("Time Ago")
                .style(Style::default().fg(THEME.dim))
                .labels(x_labels)
                .bounds([min_time, max_time]),
        )
        .y_axis(
            Axis::default()
                .title("Speed")
                .style(Style::default().fg(THEME.dim))
                .labels(y_labels)
                .bounds([0.0, max_speed]),
        );

    frame.render_widget(chart, area);

    // Floating badge showing the current speed near the top-left of the graph.
    let badge_width = format!("{arrow} {}", format_bytes(data.current_speed)).len() as u16;
    let badge_area = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: badge_width.min(area.width.saturating_sub(2)),
        height: 1,
    };
    let badge = Paragraph::new(Span::styled(
        format!("{arrow} {}", format_bytes(data.current_speed)),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(badge, badge_area);
}

/// Render the RX speed graph.
pub fn render_rx_graph(
    frame: &mut Frame,
    area: Rect,
    history: &SpeedHistory,
    rx_speed: f64,
    total_rx_bytes: u64,
) {
    let data = GraphData {
        timestamps: &history.timestamps,
        speeds: &history.rx_speeds,
        max_speed: history.max_rx(),
        current_speed: rx_speed,
        total_bytes: total_rx_bytes,
    };
    render_speed_graph(frame, area, data, "RX Speed History", THEME.rx, "\u{2193}");
}

/// Render the TX speed graph.
pub fn render_tx_graph(
    frame: &mut Frame,
    area: Rect,
    history: &SpeedHistory,
    tx_speed: f64,
    total_tx_bytes: u64,
) {
    let data = GraphData {
        timestamps: &history.timestamps,
        speeds: &history.tx_speeds,
        max_speed: history.max_tx(),
        current_speed: tx_speed,
        total_bytes: total_tx_bytes,
    };
    render_speed_graph(frame, area, data, "TX Speed History", THEME.tx, "\u{2191}");
}
