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

/// Render RX speed graph with line chart
pub fn render_rx_graph(
    frame: &mut Frame,
    area: Rect,
    history: &SpeedHistory,
    rx_speed: f64,
    peak_rx_speed: f64,
    total_rx_bytes: u64,
) {
    if history.rx_speeds.is_empty() {
        let placeholder = Paragraph::new("Collecting data...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(" RX Speed History ")
                    .title_style(Style::default().fg(Color::Cyan)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, area);
        return;
    }

    // Prepare data for chart
    let data: Vec<(f64, f64)> = history
        .timestamps
        .iter()
        .zip(history.rx_speeds.iter())
        .map(|(t, s)| (*t, *s))
        .collect();

    // Dynamic scaling - use actual max
    let max_rx = history.max_rx.max(1.0); // Ensure minimum scale

    let min_time = history.timestamps.first().copied().unwrap_or(0.0);
    let max_time = history
        .timestamps
        .last()
        .copied()
        .unwrap_or(100.0)
        .max(min_time + 1.0); // Ensure range

    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Cyan))
        .data(&data)];

    let x_labels = vec![
        Span::raw(format!("{:.0}s", min_time)),
        Span::raw(format!("{:.0}s", (min_time + max_time) / 2.0)),
        Span::raw(format!("{:.0}s", max_time)),
    ];

    let y_labels = vec![
        Span::raw("0"),
        Span::raw(format_bytes(max_rx / 2.0)),
        Span::raw(format_bytes(max_rx)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(format!(
                    " RX - Current: {} | Peak: {} | Total: {} ",
                    format_bytes(rx_speed),
                    format_bytes(peak_rx_speed),
                    format_total_bytes(total_rx_bytes)
                ))
                .title_style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
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
                .bounds([0.0, max_rx]),
        );

    frame.render_widget(chart, area);
}

/// Render TX speed graph with line chart
pub fn render_tx_graph(
    frame: &mut Frame,
    area: Rect,
    history: &SpeedHistory,
    tx_speed: f64,
    peak_tx_speed: f64,
    total_tx_bytes: u64,
) {
    if history.tx_speeds.is_empty() {
        let placeholder = Paragraph::new("Collecting data...")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Green))
                    .title(" TX Speed History ")
                    .title_style(Style::default().fg(Color::Green)),
            )
            .alignment(Alignment::Center);
        frame.render_widget(placeholder, area);
        return;
    }

    // Prepare data for chart
    let data: Vec<(f64, f64)> = history
        .timestamps
        .iter()
        .zip(history.tx_speeds.iter())
        .map(|(t, s)| (*t, *s))
        .collect();

    // Dynamic scaling - use actual max
    let max_tx = history.max_tx.max(1.0); // Ensure minimum scale

    let min_time = history.timestamps.first().copied().unwrap_or(0.0);
    let max_time = history
        .timestamps
        .last()
        .copied()
        .unwrap_or(100.0)
        .max(min_time + 1.0); // Ensure range

    let datasets = vec![Dataset::default()
        .marker(symbols::Marker::Braille)
        .graph_type(GraphType::Line)
        .style(Style::default().fg(Color::Green))
        .data(&data)];

    let x_labels = vec![
        Span::raw(format!("{:.0}s", min_time)),
        Span::raw(format!("{:.0}s", (min_time + max_time) / 2.0)),
        Span::raw(format!("{:.0}s", max_time)),
    ];

    let y_labels = vec![
        Span::raw("0"),
        Span::raw(format_bytes(max_tx / 2.0)),
        Span::raw(format_bytes(max_tx)),
    ];

    let chart = Chart::new(datasets)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Green))
                .title(format!(
                    " TX - Current: {} | Peak: {} | Total: {} ",
                    format_bytes(tx_speed),
                    format_bytes(peak_tx_speed),
                    format_total_bytes(total_tx_bytes)
                ))
                .title_style(
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ),
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
                .bounds([0.0, max_tx]),
        );

    frame.render_widget(chart, area);
}
