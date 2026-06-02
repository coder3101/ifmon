use std::fmt::Display;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use netdev::{Interface, get_interfaces, interface, stats::counters::InterfaceStats};
use ratatui::{
    DefaultTerminal, Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Style, Stylize},
    symbols,
    text::Line,
    widgets::{Block, Paragraph, Row, Table, Tabs, Wrap},
};

#[derive(Debug)]
pub struct App {
    /// Is the application running?
    running: bool,
    interfaces: Vec<Interface>,
    last_stats: Option<InterfaceStats>,
    selected_interface: usize,
}

impl App {
    /// Construct a new instance of [`App`].
    pub fn new() -> Self {
        Self {
            last_stats: None,
            running: true,
            interfaces: get_interfaces()
                .into_iter()
                .filter(|i| i.is_physical())
                .collect(),
            selected_interface: 0,
        }
    }

    fn change_interface(&mut self, idx: usize) {
        self.selected_interface = idx;
        self.last_stats = None;
    }

    fn get_selected_interface(&mut self) -> Option<Interface> {
        self.interfaces.get(self.selected_interface).cloned()
    }

    fn show_optional<T>(opt: &Option<T>) -> String
    where
        T: Display,
    {
        match opt {
            Some(n) => n.to_string(),
            None => String::from("-"),
        }
    }
    /// Run the application's main loop.
    pub fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        self.running = true;
        while self.running {
            terminal.draw(|frame| self.render(frame))?;
            self.handle_crossterm_events()?;
        }
        Ok(())
    }

    /// Renders the user interface.
    ///
    /// This is where you add new widgets. See the following resources for more information:
    ///
    /// - <https://docs.rs/ratatui/latest/ratatui/widgets/index.html>
    /// - <https://github.com/ratatui/ratatui/tree/main/ratatui-widgets/examples>
    fn render(&mut self, frame: &mut Frame) {
        let title = Line::from(format!(
            "Network Interface Monitor - v{}",
            env!("CARGO_PKG_VERSION")
        ))
        .bold()
        .blue()
        .centered();

        let tabs = Tabs::new(self.interfaces.iter().map(|i| i.name.to_string()))
            .block(Block::new().title("Interfaces (use tab for cycling)"))
            .style(Style::default().white())
            .highlight_style(Style::default().yellow())
            .select(self.selected_interface)
            .divider(symbols::DOT);

        let block = Block::bordered()
            .title(title)
            .title_bottom("Press q or esc to quit");
        let area = block.inner(frame.area());

        let layout = Layout::vertical([Constraint::Length(2), Constraint::Fill(1)]).spacing(1);
        let [top, main] = area.layout(&layout);

        frame.render_widget(block, frame.area());
        frame.render_widget(tabs, top);

        self.render_interface(frame, main);
    }

    fn render_interface(&mut self, frame: &mut Frame, area: Rect) {
        let layout = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)]);
        let [info, stats] = area.layout(&layout);

        if let Some(mut interface) = self.get_selected_interface() {
            let _ = interface.update_stats();

            // 1. Format the IP strings (standard newlines are fine here, the cell handles it)
            let ipv4_str = interface
                .ipv4
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            let ipv6_str = interface
                .ipv6
                .iter()
                .map(|ip| ip.to_string())
                .collect::<Vec<_>>()
                .join("\n");

            // 2. Calculate row heights (minimum 1, or the amount of IPs you have)
            let ipv4_height = interface.ipv4.len().max(1) as u16;
            let ipv6_height = interface.ipv6.len().max(1) as u16;
            // 3. Construct your rows
            let rows = vec![
                Row::new(vec!["Name".to_string(), interface.name]),
                Row::new(vec![
                    "Description".to_string(),
                    Self::show_optional(&interface.description),
                ]),
                Row::new(vec![
                    "Friendly name".to_string(),
                    Self::show_optional(&interface.friendly_name),
                ]),
                Row::new(vec!["Interface type".to_string(), interface.if_type.name()]),
                Row::new(vec![
                    "Mac Address".to_string(),
                    Self::show_optional(&interface.mac_addr.map(|m| m.to_string())),
                ]),
                // Pass the calculated heights to the multi-line rows
                Row::new(vec!["IPv4 Address", &ipv4_str]).height(ipv4_height),
                Row::new(vec!["IPv6 Address", &ipv6_str]).height(ipv6_height),
                Row::new(vec![
                    "Transmit Speed".to_string(),
                    Self::show_optional(&interface.transmit_speed.map(|x| x / 8)),
                ]),
                Row::new(vec![
                    "Receive Speed".to_string(),
                    Self::show_optional(&interface.receive_speed.map(|x| x / 8)),
                ]),
            ];

            // 4. Render the Table.
            // Constraint::Length(15) locks the label column width.
            // Constraint::Min(20) lets the values take up the remaining space.
            let content = Table::new(rows, [Constraint::Length(15), Constraint::Min(20)])
                .block(Block::bordered().title("Information").bold());

            frame.render_widget(content, info);

            if let Some(istats) = &interface.stats {
                self.render_stats(frame, stats, &istats);
            }
        }
    }

    fn render_stats(&self, frame: &mut Frame, area: Rect, stats: &InterfaceStats) {
        let layout = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)]);
        let [tx, rx] = area.layout(&layout);
        self.render_tx(frame, tx, stats);
        self.render_rx(frame, rx, stats);
    }

    fn render_rx(&self, frame: &mut Frame, area: Rect, interface: &InterfaceStats) {
        frame.render_widget(Block::bordered().title("Upload").bold(), area);
    }

    fn render_tx(&self, frame: &mut Frame, area: Rect, interface: &InterfaceStats) {
        frame.render_widget(Block::bordered().title("Download").bold(), area);
    }

    /// Reads the crossterm events and updates the state of [`App`].
    ///
    /// If your application needs to perform work in between handling events, you can use the
    /// [`event::poll`] function to check if there are any events available with a timeout.
    fn handle_crossterm_events(&mut self) -> color_eyre::Result<()> {
        match event::read()? {
            // it's important to check KeyEventKind::Press to avoid handling key release events
            Event::Key(key) if key.kind == KeyEventKind::Press => self.on_key_event(key),
            Event::Mouse(_) => {}
            Event::Resize(_, _) => {}
            _ => {}
        }
        Ok(())
    }

    /// Handles the key events and updates the state of [`App`].
    fn on_key_event(&mut self, key: KeyEvent) {
        match (key.modifiers, key.code) {
            (_, KeyCode::Esc | KeyCode::Char('q')) => self.quit(),
            (KeyModifiers::SHIFT, KeyCode::Tab) => {
                let new_idx = if self.selected_interface == 0 {
                    self.interfaces.len() - 1
                } else {
                    (self.selected_interface - 1) % self.interfaces.len()
                };
                self.change_interface(new_idx);
            }
            (_, KeyCode::Tab) => {
                let new_idx = (self.selected_interface + 1) % self.interfaces.len();
                self.change_interface(new_idx);
            }
            _ => {}
        }
    }

    /// Set running to false to quit the application.
    fn quit(&mut self) {
        self.running = false;
    }
}
