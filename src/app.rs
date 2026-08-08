use std::collections::HashMap;
use std::time::{Duration, Instant};

use crossterm::event::{
    self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent,
    MouseEventKind,
};
use netdev::{get_interfaces, stats::counters::InterfaceStats, Interface};
use ratatui::{
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    DefaultTerminal, Frame,
};

use crate::network::filter_interfaces;
use crate::types::SpeedHistory;
use crate::ui::theme::THEME;
use crate::ui::{
    render_help, render_interface_info, render_rx_graph, render_sidebar, render_tx_graph,
};
use crate::utils::format_total_bytes;
use unicode_width::UnicodeWidthStr;

/// Number of historical speed samples kept per interface.
const HISTORY_SIZE: usize = 100;
/// Default time between samples if none is configured.
const DEFAULT_UPDATE_INTERVAL: Duration = Duration::from_millis(500);
/// Minimum elapsed time before a new speed sample is computed.
const MIN_SAMPLE_ELAPSED: Duration = Duration::from_millis(100);
/// Minimum width for the left interface sidebar.
const SIDEBAR_MIN_WIDTH: u16 = 20;
/// Extra columns reserved for the speed block and highlight symbol/borders.
const SIDEBAR_SPEED_RESERVE: usize = 28;
/// Minimum width to keep for the main (info + graph) content.
const MAIN_MIN_WIDTH: u16 = 50;
/// Height of the bottom status bar.
const STATUS_HEIGHT: u16 = 1;
/// Height of the interface info panel (in rows).
const INFO_PANEL_HEIGHT: u16 = 14;
/// IPv4 rows visible in the info panel (drives whether arrows scroll IPv4).
const IPV4_VISIBLE_ROWS: usize = 2;

/// Main application state.
#[derive(Debug)]
pub struct App {
    running: bool,
    interfaces: Vec<Interface>,
    filtered_interfaces: Vec<Interface>,
    selected_interface: usize,
    show_help: bool,
    show_all_interfaces: bool,
    update_interval: Duration,
    /// Ring buffer of (rx, tx) speeds for the currently selected interface.
    history: SpeedHistory,
    rx_speed: f64,
    tx_speed: f64,
    total_rx_bytes: u64,
    total_tx_bytes: u64,
    ipv4_scroll_offset: usize,
    ipv6_scroll_offset: usize,
    /// When the selected graph was last pushed to, to keep pushes at sample cadence.
    last_history_update: Option<Instant>,
    /// Per-interface last snapshot used to compute deltas, keyed by interface index.
    last_stats: HashMap<u32, InterfaceStats>,
    last_sample_time: HashMap<u32, Instant>,
    /// Per-interface current (rx_speed, tx_speed), keyed by interface index.
    speeds: HashMap<u32, (f64, f64)>,
    /// Per-interface cumulative (rx_bytes, tx_bytes), keyed by interface index.
    totals: HashMap<u32, (u64, u64)>,
    /// Screen area of the sidebar (used for mouse hit-testing).
    sidebar_area: Rect,
    /// Y coordinate of the first sidebar row (inside the top border).
    sidebar_rows_top: u16,
}

impl App {
    /// The built-in default time between samples.
    pub const fn default_update_interval() -> Duration {
        DEFAULT_UPDATE_INTERVAL
    }

    /// Create a new application instance with the default update interval.
    pub fn new() -> Self {
        Self::with_update_interval(Self::default_update_interval())
    }

    /// Create a new application instance with a custom update interval.
    pub fn with_update_interval(update_interval: Duration) -> Self {
        let all_interfaces: Vec<Interface> = get_interfaces().into_iter().collect();
        let filtered = filter_interfaces(&all_interfaces);

        Self {
            running: true,
            interfaces: all_interfaces,
            filtered_interfaces: filtered,
            selected_interface: 0,
            show_help: false,
            show_all_interfaces: false,
            update_interval,
            history: SpeedHistory::new(),
            rx_speed: 0.0,
            tx_speed: 0.0,
            total_rx_bytes: 0,
            total_tx_bytes: 0,
            ipv4_scroll_offset: 0,
            ipv6_scroll_offset: 0,
            last_history_update: None,
            last_stats: HashMap::new(),
            last_sample_time: HashMap::new(),
            speeds: HashMap::new(),
            totals: HashMap::new(),
            sidebar_area: Rect::default(),
            sidebar_rows_top: 0,
        }
    }

    /// Get the list of active interfaces based on the current filter setting.
    fn get_active_interfaces(&self) -> &[Interface] {
        if self.show_all_interfaces {
            &self.interfaces
        } else {
            &self.filtered_interfaces
        }
    }

    /// Toggle between showing all interfaces and filtered interfaces.
    fn toggle_interface_filter(&mut self) {
        self.show_all_interfaces = !self.show_all_interfaces;
        self.selected_interface = 0;
        self.reset_stats();
    }

    /// Change to a different interface (no-op for out-of-range indices).
    fn change_interface(&mut self, idx: usize) {
        let len = self.get_active_interfaces().len();
        if len == 0 || idx >= len {
            return;
        }
        self.selected_interface = idx;
        self.reset_stats();
        self.ipv4_scroll_offset = 0;
        self.ipv6_scroll_offset = 0;
    }

    /// Move to the next (wrapping) interface.
    fn next_interface(&mut self) {
        let len = self.get_active_interfaces().len();
        if len == 0 {
            return;
        }
        self.change_interface((self.selected_interface + 1) % len);
    }

    /// Move to the previous (wrapping) interface.
    fn prev_interface(&mut self) {
        let len = self.get_active_interfaces().len();
        if len == 0 {
            return;
        }
        let idx = if self.selected_interface == 0 {
            len - 1
        } else {
            self.selected_interface - 1
        };
        self.change_interface(idx);
    }

    /// Reset per-selection graph statistics (keeps per-interface speed data intact).
    fn reset_stats(&mut self) {
        self.history.clear();
        self.rx_speed = 0.0;
        self.tx_speed = 0.0;
        self.total_rx_bytes = 0;
        self.total_tx_bytes = 0;
        self.last_history_update = None;
    }

    /// Refresh stats for all active interfaces and compute per-interface speeds.
    fn sample(&mut self) {
        let now = Instant::now();
        let show_all = self.show_all_interfaces;

        // Temporarily take ownership of the active list so we can mutate each
        // interface in place without cloning and without borrowing self.
        let mut active = if show_all {
            std::mem::take(&mut self.interfaces)
        } else {
            std::mem::take(&mut self.filtered_interfaces)
        };

        for iface in &mut active {
            let idx = iface.index;

            let due = match self.last_sample_time.get(&idx) {
                Some(&t) => now.duration_since(t) >= MIN_SAMPLE_ELAPSED,
                None => true,
            };
            if !due {
                continue;
            }

            let _ = iface.update_stats();

            if let Some(stats) = &iface.stats {
                // Cumulative totals are available immediately (even on first sample).
                self.totals.insert(idx, (stats.rx_bytes, stats.tx_bytes));

                if let (Some(prev), Some(t0)) = (
                    self.last_stats.get(&idx),
                    self.last_sample_time.get(&idx).copied(),
                ) {
                    let elapsed = now.duration_since(t0).as_secs_f64().max(0.001);
                    let rx = stats.rx_bytes.saturating_sub(prev.rx_bytes) as f64 / elapsed;
                    let tx = stats.tx_bytes.saturating_sub(prev.tx_bytes) as f64 / elapsed;
                    self.speeds.insert(idx, (rx, tx));
                }
                self.last_stats.insert(idx, stats.clone());
            }
            self.last_sample_time.insert(idx, now);
        }

        if show_all {
            self.interfaces = active;
        } else {
            self.filtered_interfaces = active;
        }
    }

    /// Run the application's main loop.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        self.sample();
        while self.running {
            terminal.draw(|frame| self.render(frame))?;

            if event::poll(self.update_interval)? {
                self.handle_crossterm_events()?;
                // Drain any remaining queued events (e.g. mouse-move bursts) so
                // input latency stays low and we avoid needless extra renders.
                while event::poll(Duration::ZERO)? {
                    self.handle_crossterm_events()?;
                }
            }
            self.sample();
        }
        Ok(())
    }

    /// Render the user interface.
    fn render(&mut self, frame: &mut Frame) {
        if self.show_help {
            render_help(frame);
            return;
        }

        let root = Layout::vertical([Constraint::Fill(1), Constraint::Length(STATUS_HEIGHT)])
            .split(frame.area());
        let content_area = root[0];
        let status_area = root[1];

        let title = Line::from(vec![
            Span::raw("Network Interface Monitor "),
            Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                Style::default()
                    .fg(THEME.accent)
                    .add_modifier(Modifier::DIM),
            ),
        ])
        .alignment(Alignment::Center);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(THEME.border))
            .title(title.style(Style::default().fg(THEME.text).add_modifier(Modifier::BOLD)));

        let inner = block.inner(content_area);
        frame.render_widget(block, content_area);

        // Size the sidebar to fit the longest interface name so names aren't truncated.
        let (active_len, longest_name) = {
            let active = self.get_active_interfaces();
            (
                active.len(),
                active.iter().map(|i| i.name.width()).max().unwrap_or(0),
            )
        };
        let available = inner.width.saturating_sub(MAIN_MIN_WIDTH);
        let sidebar_width = (longest_name + SIDEBAR_SPEED_RESERVE)
            .min(available as usize)
            .max(SIDEBAR_MIN_WIDTH as usize) as u16;

        let columns = Layout::horizontal([Constraint::Length(sidebar_width), Constraint::Fill(1)])
            .split(inner);
        let sidebar_area = columns[0];
        let main_area = columns[1];

        self.sidebar_area = sidebar_area;
        self.sidebar_rows_top = sidebar_area.y + 1;

        if active_len == 0 {
            self.render_empty_state(frame, main_area);
        } else {
            let active = self.get_active_interfaces();
            render_sidebar(
                frame,
                sidebar_area,
                active,
                self.selected_interface,
                &self.speeds,
                self.show_all_interfaces,
                &THEME,
            );
            self.render_interface(frame, main_area);
        }

        self.render_status_bar(frame, status_area);
    }

    /// Render a message when no interfaces are available.
    fn render_empty_state(&self, frame: &mut Frame, area: Rect) {
        let placeholder = Paragraph::new(
            "No network interfaces to display.\n\nPress 'f' to toggle showing all interfaces.",
        )
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(THEME.border))
                .title(" No Interfaces ")
                .title_style(Style::default().fg(THEME.accent)),
        );
        frame.render_widget(placeholder, area);
    }

    /// Reflect the selected interface's latest metrics into the graph state.
    fn update_selected_metrics(&mut self) {
        let Some(iface) = self.get_active_interfaces().get(self.selected_interface) else {
            return;
        };
        let idx = iface.index;

        if let Some(&(rx, tx)) = self.speeds.get(&idx) {
            self.rx_speed = rx;
            self.tx_speed = tx;

            // Push to history at the sample cadence, never on every draw (mouse
            // movement would otherwise compress the graph by flooding renders).
            let now = Instant::now();
            let (due, delta) = match self.last_history_update {
                Some(t) => (
                    now.duration_since(t) >= MIN_SAMPLE_ELAPSED,
                    now.duration_since(t).as_secs_f64(),
                ),
                None => (true, 0.0),
            };
            if due {
                self.last_history_update = Some(now);
                self.history.push(rx, tx, delta, HISTORY_SIZE);
            }
        }
        if let Some(&(r, t)) = self.totals.get(&idx) {
            self.total_rx_bytes = r;
            self.total_tx_bytes = t;
        }
    }

    /// Render the interface details and graphs.
    fn render_interface(&mut self, frame: &mut Frame, area: Rect) {
        self.update_selected_metrics();

        let layout = Layout::vertical([Constraint::Length(INFO_PANEL_HEIGHT), Constraint::Fill(1)])
            .split(area);
        let top_area = layout[0];
        let bottom_area = layout[1];

        if let Some(interface) = self.get_active_interfaces().get(self.selected_interface) {
            render_interface_info(
                frame,
                top_area,
                interface,
                self.ipv4_scroll_offset,
                self.ipv6_scroll_offset,
            );
        }
        self.render_graphs_section(frame, bottom_area);
    }

    /// Render the speed graphs section.
    fn render_graphs_section(&self, frame: &mut Frame, area: Rect) {
        let layout =
            Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]).split(area);
        let rx_graph_area = layout[0];
        let tx_graph_area = layout[1];

        render_rx_graph(
            frame,
            rx_graph_area,
            &self.history,
            self.rx_speed,
            self.total_rx_bytes,
        );

        render_tx_graph(
            frame,
            tx_graph_area,
            &self.history,
            self.tx_speed,
            self.total_tx_bytes,
        );
    }

    /// Render the bottom status bar.
    fn render_status_bar(&self, frame: &mut Frame, area: Rect) {
        let name = self
            .get_active_interfaces()
            .get(self.selected_interface)
            .map(|i| i.name.clone())
            .unwrap_or_else(|| String::from("none"));

        let mode = if self.show_all_interfaces {
            "all"
        } else {
            "physical"
        };

        let line = Line::from(vec![
            Span::styled(format!(" {name} "), Style::default().fg(THEME.accent)),
            Span::styled(format!(" mode:{mode} "), Style::default().fg(THEME.dim)),
            Span::styled(
                format!(" {}ms ", self.update_interval.as_millis()),
                Style::default().fg(THEME.dim),
            ),
            Span::styled(
                format!(" \u{2193}{} ", format_total_bytes(self.total_rx_bytes)),
                Style::default().fg(THEME.rx),
            ),
            Span::styled(
                format!(" \u{2191}{} ", format_total_bytes(self.total_tx_bytes)),
                Style::default().fg(THEME.tx),
            ),
            Span::styled(
                "  [q]quit [Tab/j]next [k]prev [1-9]jump [h]help [f]filter ",
                Style::default().fg(THEME.dim),
            ),
        ]);

        frame.render_widget(Paragraph::new(line), area);
    }

    /// Read and handle crossterm events.
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(mouse) => self.on_mouse_event(mouse),
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Scroll the IP address list. Scrolls whichever family (IPv4 then IPv6) still
    /// has addresses beyond what's visible, instead of scrolling both at once.
    fn scroll_ip_addresses(&mut self, down: bool) {
        if self.show_help {
            return;
        }
        let v4_overflow = {
            let iface = self.get_active_interfaces().get(self.selected_interface);
            match iface {
                Some(i) => i.ipv4.len() > IPV4_VISIBLE_ROWS,
                None => return,
            }
        };
        let offset = if v4_overflow {
            &mut self.ipv4_scroll_offset
        } else {
            &mut self.ipv6_scroll_offset
        };
        if down {
            *offset += 1;
        } else {
            *offset = offset.saturating_sub(1);
        }
    }

    /// Handle mouse events.
    fn on_mouse_event(&mut self, mouse: MouseEvent) {
        if self.show_help {
            return;
        }
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let in_sidebar = mouse.column >= self.sidebar_area.x
                    && mouse.column < self.sidebar_area.x.saturating_add(self.sidebar_area.width)
                    && mouse.row >= self.sidebar_rows_top
                    && mouse.row < self.sidebar_area.y.saturating_add(self.sidebar_area.height);
                if in_sidebar {
                    let idx = (mouse.row - self.sidebar_rows_top) as usize;
                    if idx < self.get_active_interfaces().len() {
                        self.change_interface(idx);
                    }
                }
            }
            MouseEventKind::ScrollDown => self.scroll_ip_addresses(true),
            MouseEventKind::ScrollUp => self.scroll_ip_addresses(false),
            _ => {}
        }
    }

    /// Handle key events.
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc) if self.show_help => {
                self.show_help = false;
            }
            (_, KeyCode::Esc | KeyCode::Char('q')) => self.quit(),
            (_, KeyCode::Char('h')) => {
                self.show_help = !self.show_help;
            }
            (_, KeyCode::Char('f')) => {
                self.toggle_interface_filter();
            }
            (_, KeyCode::Up) => self.scroll_ip_addresses(false),
            (_, KeyCode::Down) => self.scroll_ip_addresses(true),
            (KeyModifiers::SHIFT, KeyCode::Tab) => self.prev_interface(),
            (_, KeyCode::Tab) => self.next_interface(),
            (_, KeyCode::Char('k')) => self.prev_interface(),
            (_, KeyCode::Char('j')) => self.next_interface(),
            (_, KeyCode::Char(c)) if c.is_ascii_digit() && c != '0' => {
                let idx = (c as u8 - b'1') as usize;
                self.change_interface(idx);
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
