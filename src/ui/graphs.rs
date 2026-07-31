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

/// Data needed to render one speed history graph.
struct GraphData<'a> {
    timestamps: &'a VecDeque<f64>,
    speeds: &'a VecDeque<f64>,
    max_speed: f64,
    current_speed: f64,
    peak_speed: f64,
    total_bytes: u64,
}

/// Render a single speed history graph. Used for both RX and TX.
fn render_speed_graph(frame: &mut Frame, area: Rect, data: GraphData, title: &str, color: Color) {
    if data.speeds.is_empty() {
        let placeholder = Paragraph::new("Collecting data...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(color))
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

    let x_labels = vec![
        Span::raw(format!("{:.0}s", min_time)),
        Span::raw(format!("{:.0}s", (min_time + max_time) / 2.0)),
        Span::raw(format!("{:.0}s", max_time)),
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
                .border_style(Style::default().fg(color))
                .title(format!(
                    " {title} - Current: {} | Peak: {} | Total: {} ",
                    format_bytes(data.current_speed),
                    format_bytes(data.peak_speed),
                    format_total_bytes(data.total_bytes)
                ))
                .title_style(Style::default().fg(color).add_modifier(Modifier::BOLD)),
        )
        .x_axis(
            Axis::default()
                .title("Time")
                .style(Style::default().fg(Color::DarkGray))
                .labels(x_labels)
                .bounds([min_time, max_time]),
        )
        .y_axis(
            Axis::default()
                .title("Speed")
                .style(Style::default().fg(Color::DarkGray))
                .labels(y_labels)
                .bounds([0.0, max_speed]),
        );

    frame.render_widget(chart, area);
}

/// Render the RX speed graph.
pub fn render_rx_graph(
    frame: &mut Frame,
    area: Rect,
    history: &SpeedHistory,
    rx_speed: f64,
    peak_rx_speed: f64,
    total_rx_bytes: u64,
) {
    let data = GraphData {
        timestamps: &history.timestamps,
        speeds: &history.rx_speeds,
        max_speed: history.max_rx(),
        current_speed: rx_speed,
        peak_speed: peak_rx_speed,
        total_bytes: total_rx_bytes,
    };
    render_speed_graph(frame, area, data, "RX Speed History", Color::Cyan);
}

/// Render the TX speed graph.
pub fn render_tx_graph(
    frame: &mut Frame,
    area: Rect,
    history: &SpeedHistory,
    tx_speed: f64,
    peak_tx_speed: f64,
    total_tx_bytes: u64,
) {
    let data = GraphData {
        timestamps: &history.timestamps,
        speeds: &history.tx_speeds,
        max_speed: history.max_tx(),
        current_speed: tx_speed,
        peak_speed: peak_tx_speed,
        total_bytes: total_tx_bytes,
    };
    render_speed_graph(frame, area, data, "TX Speed History", Color::Green);
}
