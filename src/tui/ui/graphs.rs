use std::time::Duration;

use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Axis, Chart, Dataset, GraphType, LegendPosition, Paragraph};

use super::panel;
use super::stats::window_summary;
use crate::net::{Point, Track};
use crate::settings::{GraphStyle, GraphView};
use crate::theme::Theme;
use crate::tui::app::App;
use crate::units::{self, Units, nice_ceil, short_duration};

/// Split views fall back to a single overlaid chart below this height.
const SPLIT_MIN_HEIGHT: u16 = 14;

#[derive(Clone, Copy)]
enum Metric {
    Bytes,
    Packets,
}

impl Metric {
    fn value(self, p: &Point, rx: bool) -> f64 {
        match (self, rx) {
            (Self::Bytes, true) => p.rx,
            (Self::Bytes, false) => p.tx,
            (Self::Packets, true) => p.rx_packets,
            (Self::Packets, false) => p.tx_packets,
        }
    }

    fn format(self, units: Units, v: f64) -> String {
        match self {
            Self::Bytes => units.rate(v),
            Self::Packets => format!("{} pkt/s", units::count(v)),
        }
    }

    fn axis_max(self, units: Units, v: f64) -> f64 {
        match self {
            Self::Bytes => units.nice_axis(v),
            Self::Packets => nice_ceil(v),
        }
    }
}

struct Series {
    name: &'static str,
    color: Color,
    points: Vec<(f64, f64)>,
}

fn style_parts(style: GraphStyle) -> (Marker, GraphType) {
    match style {
        GraphStyle::Braille => (Marker::Braille, GraphType::Line),
        GraphStyle::Block => (Marker::HalfBlock, GraphType::Line),
        GraphStyle::Bars => (Marker::Braille, GraphType::Bar),
        GraphStyle::Dots => (Marker::Braille, GraphType::Scatter),
    }
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let theme = &app.theme;
    let Some(track) = app.selected_track().filter(|t| t.history.len() >= 2) else {
        let placeholder = Paragraph::new("\n\nCollecting data\u{2026}")
            .alignment(Alignment::Center)
            .style(Style::new().fg(theme.dim))
            .block(panel(theme, " Throughput ", false));
        frame.render_widget(placeholder, area);
        return;
    };

    let metric = match app.view {
        GraphView::Packets if app.data().extended() => Metric::Packets,
        _ => Metric::Bytes,
    };
    let split = app.view != GraphView::Combined && area.height >= SPLIT_MIN_HEIGHT;

    if split {
        let [top, bottom] =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).areas(area);
        render_chart(frame, top, app, track, metric, &[true]);
        render_chart(frame, bottom, app, track, metric, &[false]);
    } else {
        render_chart(frame, area, app, track, metric, &[true, false]);
    }
}

fn render_chart(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    track: &Track,
    metric: Metric,
    dirs: &[bool],
) {
    let theme = &app.theme;
    let now = app.data().now();
    let window = app.window.as_secs_f64();
    let since = now - window;

    let series: Vec<Series> = dirs
        .iter()
        .map(|&rx| Series {
            name: if rx { "\u{2193} rx" } else { "\u{2191} tx" },
            color: if rx { theme.rx } else { theme.tx },
            points: track
                .history
                .since(since)
                .map(|p| (p.t - now, metric.value(p, rx)))
                .collect(),
        })
        .collect();

    let peak = series
        .iter()
        .flat_map(|s| s.points.iter().map(|&(_, v)| v))
        .fold(0.0, f64::max);
    let y_max = metric.axis_max(app.units, peak * 1.05);

    let single = dirs.len() == 1;
    // A dotted average line, only where it can't be confused with another series.
    let averages: Vec<[(f64, f64); 2]> = if single {
        series
            .iter()
            .map(|s| {
                let avg =
                    s.points.iter().map(|&(_, v)| v).sum::<f64>() / s.points.len().max(1) as f64;
                [(-window, avg), (0.0, avg)]
            })
            .collect()
    } else {
        Vec::new()
    };

    let (marker, graph_type) = style_parts(app.style);
    let mut datasets: Vec<Dataset> = averages
        .iter()
        .map(|pts| {
            Dataset::default()
                .marker(Marker::Braille)
                .graph_type(GraphType::Line)
                .style(Style::new().fg(theme.dim))
                .data(pts)
        })
        .collect();
    datasets.extend(series.iter().map(|s| {
        let ds = Dataset::default()
            .marker(marker)
            .graph_type(graph_type)
            .style(Style::new().fg(s.color))
            .data(&s.points);
        if single { ds } else { ds.name(s.name) }
    }));

    let title = title(app, track, metric, dirs);
    let subtitle = subtitle(app, track, metric, dirs, since);
    let mut block = panel(theme, title.clone(), false);
    // Both titles share the top border; drop the summary rather than let them collide.
    if title.width() + subtitle.width() + 4 <= usize::from(area.width) {
        block = block.title_top(subtitle.right_aligned());
    }

    let half = Duration::from_secs_f64(window / 2.0);
    let x_labels = vec![
        Span::raw(format!("-{}", short_duration(app.window))),
        Span::raw(format!("-{}", short_duration(half))),
        Span::raw("now"),
    ];
    let y_labels = vec![
        Span::raw(metric.format(app.units, 0.0)),
        Span::raw(metric.format(app.units, y_max / 2.0)),
        Span::raw(metric.format(app.units, y_max)),
    ];

    let chart = Chart::new(datasets)
        .block(block)
        .legend_position(Some(LegendPosition::TopLeft))
        .hidden_legend_constraints((Constraint::Ratio(1, 3), Constraint::Ratio(1, 3)))
        .x_axis(
            Axis::default()
                .style(Style::new().fg(theme.dim))
                .labels(x_labels)
                .bounds([-window, 0.0]),
        )
        .y_axis(
            Axis::default()
                .style(Style::new().fg(theme.dim))
                .labels(y_labels)
                .bounds([0.0, y_max]),
        );
    frame.render_widget(chart, area);
}

fn title<'a>(app: &App, track: &Track, metric: Metric, dirs: &[bool]) -> Line<'a> {
    let theme = &app.theme;
    let mut spans = vec![Span::raw(" ")];
    for &rx in dirs {
        let (label, color, current) = match (rx, metric) {
            (true, Metric::Bytes) => ("\u{2193} Download ", theme.rx, track.rate.rx_bytes),
            (false, Metric::Bytes) => ("\u{2191} Upload ", theme.tx, track.rate.tx_bytes),
            (true, Metric::Packets) => ("\u{2193} RX packets ", theme.rx, track.rate.rx_packets),
            (false, Metric::Packets) => ("\u{2191} TX packets ", theme.tx, track.rate.tx_packets),
        };
        spans.push(Span::styled(label, Style::new().fg(color).bold()));
        spans.push(Span::styled(
            format!("{} ", metric.format(app.units, current)),
            Style::new().fg(theme.text).bold(),
        ));
        spans.push(Span::raw(" "));
    }
    Line::from(spans)
}

fn subtitle<'a>(app: &App, track: &Track, metric: Metric, dirs: &[bool], since: f64) -> Line<'a> {
    let theme: &Theme = &app.theme;
    if !matches!(metric, Metric::Bytes) || dirs.len() != 1 {
        return Line::from(Span::styled(
            format!(" {} \u{b7} {} ", app.view.label(), app.style.label()),
            Style::new().fg(theme.dim),
        ));
    }
    let s = window_summary(track, since, dirs[0]);
    let fmt = |v| app.units.rate(v);
    Line::from(vec![
        Span::styled(" avg ", Style::new().fg(theme.dim)),
        Span::styled(fmt(s.avg), Style::new().fg(theme.text)),
        Span::styled(" \u{b7} peak ", Style::new().fg(theme.dim)),
        Span::styled(fmt(s.max), Style::new().fg(theme.text)),
        Span::styled(" \u{b7} p95 ", Style::new().fg(theme.dim)),
        Span::styled(format!("{} ", fmt(s.p95)), Style::new().fg(theme.text)),
    ])
}
