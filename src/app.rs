use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use netdev::{get_interfaces, Interface, stats::counters::InterfaceStats};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Tabs},
};

use crate::network::filter_interfaces;
use crate::types::SpeedHistory;
use crate::ui::{render_help, render_interface_info, render_rx_graph, render_tx_graph};

const HISTORY_SIZE: usize = 100;
const UPDATE_INTERVAL: Duration = Duration::from_millis(500);

/// Main application state
#[derive(Debug)]
pub struct App {
    running: bool,
    interfaces: Vec<Interface>,
    filtered_interfaces: Vec<Interface>,
    last_stats: Option<InterfaceStats>,
    last_update_time: Option<Instant>,
    selected_interface: usize,
    show_help: bool,
    show_all_interfaces: bool,
    rx_speed: f64,
    tx_speed: f64,
    history: SpeedHistory,
    total_rx_bytes: u64,
    total_tx_bytes: u64,
    peak_rx_speed: f64,
    peak_tx_speed: f64,
    ip_scroll_offset: usize,
}

impl App {
    /// Create a new application instance
    pub fn new() -> Self {
        let all_interfaces: Vec<Interface> = get_interfaces().into_iter().collect();
        let filtered = filter_interfaces(&all_interfaces);
        
        Self {
            last_stats: None,
            last_update_time: None,
            running: true,
            interfaces: all_interfaces,
            filtered_interfaces: filtered,
            selected_interface: 0,
            show_help: false,
            show_all_interfaces: false,
            rx_speed: 0.0,
            tx_speed: 0.0,
            history: SpeedHistory::new(HISTORY_SIZE),
            total_rx_bytes: 0,
            total_tx_bytes: 0,
            peak_rx_speed: 0.0,
            peak_tx_speed: 0.0,
            ip_scroll_offset: 0,
        }
    }
    
    /// Get the list of active interfaces based on filter setting
    fn get_active_interfaces(&self) -> &Vec<Interface> {
        if self.show_all_interfaces {
            &self.interfaces
        } else {
            &self.filtered_interfaces
        }
    }
    
    /// Toggle between showing all interfaces and filtered interfaces
    fn toggle_interface_filter(&mut self) {
        self.show_all_interfaces = !self.show_all_interfaces;
        self.selected_interface = 0;
        self.reset_stats();
    }

    /// Change to a different interface
    fn change_interface(&mut self, idx: usize) {
        self.selected_interface = idx;
        self.reset_stats();
        self.ip_scroll_offset = 0;
    }
    
    /// Reset all statistics
    fn reset_stats(&mut self) {
        self.last_stats = None;
        self.last_update_time = None;
        self.history.clear();
        self.rx_speed = 0.0;
        self.tx_speed = 0.0;
        self.total_rx_bytes = 0;
        self.total_tx_bytes = 0;
        self.peak_rx_speed = 0.0;
        self.peak_tx_speed = 0.0;
    }

    /// Get the currently selected interface
    fn get_selected_interface(&mut self) -> Option<Interface> {
        self.get_active_interfaces().get(self.selected_interface).cloned()
    }

    /// Run the application's main loop
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            
            // Poll for events with timeout to allow UI updates
            if event::poll(UPDATE_INTERVAL)? {
                self.handle_crossterm_events()?;
            }
        }
        Ok(())
    }

    /// Render the user interface
    fn render(&mut self, frame: &mut Frame) {
        if self.show_help {
            render_help(frame);
            return;
        }

        let title = Line::from(vec![
            Span::raw("Network Interface Monitor "),
            Span::styled(
                format!("v{}", env!("CARGO_PKG_VERSION")),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::DIM),
            ),
        ])
        .bold()
        .blue()
        .centered();

        let tabs = Tabs::new(self.get_active_interfaces().iter().map(|i| i.name.to_string()))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(if self.show_all_interfaces {
                        " All Interfaces "
                    } else {
                        " Physical Interfaces "
                    })
            )
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::DarkGray)
            )
            .select(self.selected_interface)
            .divider(symbols::DOT);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(title)
            .title_bottom(
                Line::from(" [q]uit | [Tab] Switch | [h]elp | [f]ilter ")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::DarkGray))
            );
        let area = block.inner(frame.area());

        let layout = Layout::vertical([Constraint::Length(3), Constraint::Fill(1)]);
        let sections = layout.split(area);
        let top = sections[0];
        let main = sections[1];

        frame.render_widget(block, frame.area());
        frame.render_widget(tabs, top);

        self.render_interface(frame, main);
    }

    /// Render the interface details and graphs
    fn render_interface(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([
            Constraint::Length(14),
            Constraint::Fill(1),
        ]);
        let sections = layout.split(area);
        let top_area = sections[0];
        let bottom_area = sections[1];

        if let Some(mut interface) = self.get_selected_interface() {
            // Update stats FIRST
            let _ = interface.update_stats();
            
            let now = Instant::now();
            
            // Then calculate speed based on updated stats
            if let Some(last_stats) = &self.last_stats {
                if let Some(last_time) = self.last_update_time {
                    let elapsed = now.duration_since(last_time).as_secs_f64();
                    if elapsed > 0.1 { // Only update if enough time has passed
                        if let Some(current_stats) = &interface.stats {
                            let rx_diff = current_stats.rx_bytes.saturating_sub(last_stats.rx_bytes) as f64;
                            let tx_diff = current_stats.tx_bytes.saturating_sub(last_stats.tx_bytes) as f64;
                            self.rx_speed = rx_diff / elapsed;
                            self.tx_speed = tx_diff / elapsed;
                            
                            self.peak_rx_speed = self.peak_rx_speed.max(self.rx_speed);
                            self.peak_tx_speed = self.peak_tx_speed.max(self.tx_speed);
                            
                            self.history.push(self.rx_speed, self.tx_speed, HISTORY_SIZE);
                            
                            // Update the last stats and time
                            self.last_stats = interface.stats.clone();
                            self.last_update_time = Some(now);
                        }
                    }
                }
            } else {
                // First time - just save the stats
                self.last_stats = interface.stats.clone();
                self.last_update_time = Some(now);
            }
            
            if let Some(stats) = &interface.stats {
                self.total_rx_bytes = stats.rx_bytes;
                self.total_tx_bytes = stats.tx_bytes;
            }

            render_interface_info(frame, top_area, &interface, self.ip_scroll_offset);
            self.render_graphs_section(frame, bottom_area);
        }
    }

    /// Render the speed graphs section
    fn render_graphs_section(&self, frame: &mut Frame, area: Rect) {
        let layout = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let sections = layout.split(area);
        let rx_graph_area = sections[0];
        let tx_graph_area = sections[1];

        render_rx_graph(
            frame,
            rx_graph_area,
            &self.history,
            self.rx_speed,
            self.peak_rx_speed,
            self.total_rx_bytes,
        );
        
        render_tx_graph(
            frame,
            tx_graph_area,
            &self.history,
            self.tx_speed,
            self.peak_tx_speed,
            self.total_tx_bytes,
        );
    }

    /// Read and handle crossterm events
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handle key events
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
            (_, KeyCode::Up) => {
                if self.ip_scroll_offset > 0 {
                    self.ip_scroll_offset -= 1;
                }
            }
            (_, KeyCode::Down) => {
                self.ip_scroll_offset += 1;
            }
            (KeyModifiers::SHIFT, KeyCode::Tab) => {
                let len = self.get_active_interfaces().len();
                let new_idx = if self.selected_interface == 0 {
                    len - 1
                } else {
                    (self.selected_interface - 1) % len
                };
                self.change_interface(new_idx);
            }
            (_, KeyCode::Tab) => {
                let len = self.get_active_interfaces().len();
                let new_idx = (self.selected_interface + 1) % len;
                self.change_interface(new_idx);
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application
    fn quit(&mut self) {
        self.running = false;
    }
}
