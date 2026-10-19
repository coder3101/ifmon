use std::time::{Duration, Instant};

use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};
use netdev::Interface;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::ListState;

use crate::net::{Monitor, Track, is_relevant, matches_exact, matches_query};
use crate::settings::{GraphStyle, GraphView, Settings};
use crate::theme::Theme;
use crate::units::{Units, short_duration};

use super::export;

/// Graph windows offered when zooming with `+`/`-`.
const ZOOM_LEVELS: [Duration; 9] = [
    Duration::from_secs(10),
    Duration::from_secs(30),
    Duration::from_secs(60),
    Duration::from_secs(2 * 60),
    Duration::from_secs(5 * 60),
    Duration::from_secs(10 * 60),
    Duration::from_secs(15 * 60),
    Duration::from_secs(20 * 60),
    Duration::from_secs(30 * 60),
];

const TOAST_TTL: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortKey {
    /// Order reported by the OS.
    #[default]
    System,
    Name,
    Rx,
    Tx,
    Total,
}

impl SortKey {
    fn next(self) -> Self {
        match self {
            Self::System => Self::Name,
            Self::Name => Self::Rx,
            Self::Rx => Self::Tx,
            Self::Tx => Self::Total,
            Self::Total => Self::System,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Name => "name",
            Self::Rx => "rx \u{2193}",
            Self::Tx => "tx \u{2193}",
            Self::Total => "total \u{2193}",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Filter,
    Help,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastKind {
    Info,
    Error,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub text: String,
    pub kind: ToastKind,
    until: Instant,
}

/// Screen regions recorded during rendering, used for mouse hit-testing.
#[derive(Debug, Clone, Copy, Default)]
pub struct Areas {
    pub sidebar: Rect,
    /// First row of sidebar list items (inside the border).
    pub sidebar_list: Rect,
    pub details: Rect,
    pub graphs: Rect,
}

pub struct App {
    pub monitor: Monitor,
    /// Snapshot shown while paused; sampling continues in `monitor` underneath.
    pub frozen: Option<Monitor>,
    pub interval: Duration,
    pub window: Duration,
    pub units: Units,
    pub theme: Theme,
    pub view: GraphView,
    pub style: GraphStyle,
    pub show_all: bool,
    pub sort: SortKey,
    pub filter: String,
    pub mode: Mode,
    pub selected: Option<String>,
    pub list_state: ListState,
    pub show_details: bool,
    pub details_scroll: u16,
    pub toast: Option<Toast>,
    pub areas: Areas,
    pub mouse: bool,
    pub config_path: Option<std::path::PathBuf>,
    pub running: bool,
}

impl App {
    pub fn new(settings: Settings) -> Self {
        let monitor = Monitor::new(settings.history_capacity());
        Self::with_monitor(settings, monitor)
    }

    pub fn with_monitor(settings: Settings, monitor: Monitor) -> Self {
        let mut app = Self {
            monitor,
            frozen: None,
            interval: settings.interval,
            window: settings.window,
            units: settings.units,
            theme: settings.theme,
            view: settings.view,
            style: settings.style,
            show_all: settings.show_all,
            sort: SortKey::default(),
            filter: String::new(),
            mode: Mode::Normal,
            selected: None,
            list_state: ListState::default(),
            show_details: true,
            details_scroll: 0,
            toast: None,
            areas: Areas::default(),
            mouse: settings.mouse,
            config_path: settings.config_path,
            running: true,
        };
        app.apply_focus(&settings.focus);
        app
    }

    /// Select the requested interface, or else the one carrying the default route.
    fn apply_focus(&mut self, focus: &[String]) {
        if let Some(name) = focus.first() {
            let found = self
                .monitor
                .interfaces()
                .iter()
                .find(|i| matches_exact(i, name))
                .map(|i| (i.name.clone(), is_relevant(i)));
            match found {
                Some((iface, relevant)) => {
                    self.show_all |= !relevant;
                    self.selected = Some(iface);
                }
                None => self.notify(ToastKind::Error, format!("No interface named '{name}'")),
            }
            return;
        }
        self.selected = self
            .monitor
            .interfaces()
            .iter()
            .find(|i| i.default && (self.show_all || is_relevant(i)))
            .map(|i| i.name.clone());
    }

    /// The monitor to display: the frozen snapshot while paused, otherwise live data.
    pub fn data(&self) -> &Monitor {
        self.frozen.as_ref().unwrap_or(&self.monitor)
    }

    pub fn paused(&self) -> bool {
        self.frozen.is_some()
    }

    /// Interfaces in the sidebar after filtering and sorting.
    pub fn visible(&self) -> Vec<&Interface> {
        let data = self.data();
        let mut list: Vec<&Interface> = data
            .interfaces()
            .iter()
            .filter(|i| self.show_all || is_relevant(i))
            .filter(|i| self.filter.is_empty() || matches_query(i, &self.filter))
            .collect();
        let rate = |i: &Interface, f: fn(&Track) -> f64| data.track(&i.name).map_or(0.0, f);
        match self.sort {
            SortKey::System => {}
            SortKey::Name => list.sort_by_key(|i| i.name.to_lowercase()),
            SortKey::Rx => list.sort_by(|a, b| {
                rate(b, |t| t.rate.rx_bytes).total_cmp(&rate(a, |t| t.rate.rx_bytes))
            }),
            SortKey::Tx => list.sort_by(|a, b| {
                rate(b, |t| t.rate.tx_bytes).total_cmp(&rate(a, |t| t.rate.tx_bytes))
            }),
            SortKey::Total => {
                let total = |t: &Track| t.rate.rx_bytes + t.rate.tx_bytes;
                list.sort_by(|a, b| rate(b, total).total_cmp(&rate(a, total)));
            }
        }
        list
    }

    fn visible_names(&self) -> Vec<String> {
        self.visible().into_iter().map(|i| i.name.clone()).collect()
    }

    /// Index of the selected interface within `visible()`, fixing up a stale selection.
    pub fn sync_selection(&mut self) -> Option<usize> {
        let names = self.visible_names();
        let idx = self
            .selected
            .as_ref()
            .and_then(|s| names.iter().position(|n| n == s));
        let idx = match idx {
            Some(i) => Some(i),
            None if names.is_empty() => None,
            None => {
                self.selected = Some(names[0].clone());
                self.details_scroll = 0;
                Some(0)
            }
        };
        self.list_state.select(idx);
        idx
    }

    pub fn selected_interface(&self) -> Option<&Interface> {
        let name = self.selected.as_ref()?;
        self.data().interfaces().iter().find(|i| &i.name == name)
    }

    pub fn selected_track(&self) -> Option<&Track> {
        self.data().track(self.selected.as_ref()?)
    }

    fn select_index(&mut self, idx: usize) {
        let names = self.visible_names();
        if let Some(name) = names.get(idx) {
            if self.selected.as_ref() != Some(name) {
                self.details_scroll = 0;
            }
            self.selected = Some(name.clone());
        }
    }

    fn step_selection(&mut self, forward: bool) {
        let len = self.visible_names().len();
        if len == 0 {
            return;
        }
        let cur = self.sync_selection().unwrap_or(0);
        let next = if forward {
            (cur + 1) % len
        } else {
            (cur + len - 1) % len
        };
        self.select_index(next);
    }

    pub fn notify(&mut self, kind: ToastKind, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            kind,
            until: Instant::now() + TOAST_TTL,
        });
    }

    /// Drop an expired toast. Returns `true` if the screen needs redrawing.
    pub fn expire_toast(&mut self) -> bool {
        if self
            .toast
            .as_ref()
            .is_some_and(|t| Instant::now() >= t.until)
        {
            self.toast = None;
            return true;
        }
        false
    }

    /// Longest window the stored history can fill.
    fn max_window(&self) -> Duration {
        let cap = crate::settings::history_capacity(self.interval);
        self.interval.mul_f64(cap.saturating_sub(2) as f64)
    }

    fn zoom(&mut self, zoom_in: bool) {
        let max = self.max_window();
        let next = if zoom_in {
            ZOOM_LEVELS.iter().rev().find(|&&w| w < self.window)
        } else {
            ZOOM_LEVELS.iter().find(|&&w| w > self.window && w <= max)
        };
        if let Some(&w) = next {
            self.window = w;
            self.notify(ToastKind::Info, format!("Window: {}", short_duration(w)));
        }
    }

    fn toggle_pause(&mut self) {
        if self.frozen.take().is_none() {
            self.frozen = Some(self.monitor.clone());
            self.notify(
                ToastKind::Info,
                "Paused (sampling continues in the background)",
            );
        } else {
            self.notify(ToastKind::Info, "Resumed");
        }
    }

    fn reset(&mut self) {
        self.monitor.reset();
        if let Some(f) = self.frozen.as_mut() {
            f.reset();
        }
        self.notify(ToastKind::Info, "Session statistics reset");
    }

    fn export(&mut self) {
        let Some(iface) = self.selected_interface() else {
            return;
        };
        let name = iface.name.clone();
        let Some(track) = self.data().track(&name).filter(|t| !t.history.is_empty()) else {
            self.notify(ToastKind::Error, "Nothing to export yet");
            return;
        };
        match export::write_csv(&name, track, self.data().now(), std::path::Path::new(".")) {
            Ok((path, rows)) => self.notify(
                ToastKind::Info,
                format!("Exported {rows} samples to {}", path.display()),
            ),
            Err(e) => self.notify(ToastKind::Error, format!("Export failed: {e}")),
        }
    }

    fn scroll_details(&mut self, down: bool, amount: u16) {
        self.details_scroll = if down {
            self.details_scroll.saturating_add(amount)
        } else {
            self.details_scroll.saturating_sub(amount)
        };
    }

    pub fn handle_event(&mut self, event: &Event) {
        match event {
            Event::Key(key) if key.kind != KeyEventKind::Release => self.on_key(*key),
            Event::Mouse(mouse) if self.mouse => self.on_mouse(*mouse),
            _ => {}
        }
    }

    fn on_key(&mut self, key: KeyEvent) {
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }
        match self.mode {
            Mode::Help => match key.code {
                KeyCode::Char('q') => self.running = false,
                KeyCode::Esc | KeyCode::Char('h' | '?') | KeyCode::Enter => {
                    self.mode = Mode::Normal;
                }
                _ => {}
            },
            Mode::Filter => match key.code {
                KeyCode::Esc => {
                    self.filter.clear();
                    self.mode = Mode::Normal;
                }
                KeyCode::Enter => self.mode = Mode::Normal,
                KeyCode::Backspace => {
                    self.filter.pop();
                }
                KeyCode::Down | KeyCode::Tab => self.step_selection(true),
                KeyCode::Up | KeyCode::BackTab => self.step_selection(false),
                KeyCode::Char(c) => self.filter.push(c),
                _ => {}
            },
            Mode::Normal => self.on_normal_key(key),
        }
    }

    fn on_normal_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc if !self.filter.is_empty() => self.filter.clear(),
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Char('h' | '?') => self.mode = Mode::Help,
            KeyCode::Char('/') => self.mode = Mode::Filter,

            KeyCode::Down | KeyCode::Tab | KeyCode::Char('j') => self.step_selection(true),
            KeyCode::Up | KeyCode::BackTab | KeyCode::Char('k') => self.step_selection(false),
            KeyCode::Home | KeyCode::Char('g') => self.select_index(0),
            KeyCode::End | KeyCode::Char('G') => {
                let len = self.visible_names().len();
                self.select_index(len.saturating_sub(1));
            }
            KeyCode::Char(c @ '1'..='9') => self.select_index(c as usize - '1' as usize),
            KeyCode::PageDown | KeyCode::Char('J') => self.scroll_details(true, 1),
            KeyCode::PageUp | KeyCode::Char('K') => self.scroll_details(false, 1),

            KeyCode::Char('f' | 'a') => {
                self.show_all = !self.show_all;
                let label = if self.show_all {
                    "all"
                } else {
                    "physical only"
                };
                self.notify(ToastKind::Info, format!("Showing {label} interfaces"));
            }
            KeyCode::Char('s') => {
                self.sort = self.sort.next();
                self.notify(ToastKind::Info, format!("Sort: {}", self.sort.label()));
            }
            KeyCode::Char('b') => {
                self.units.toggle_mode();
                self.notify(ToastKind::Info, format!("Units: {}", self.units.label()));
            }
            KeyCode::Char('v') => {
                self.view = self.view.next();
                self.notify(ToastKind::Info, format!("View: {}", self.view.label()));
            }
            KeyCode::Char('m') => {
                self.style = self.style.next();
                self.notify(ToastKind::Info, format!("Style: {}", self.style.label()));
            }
            KeyCode::Char('t') => {
                self.theme = self.theme.next_preset();
                self.notify(ToastKind::Info, format!("Theme: {}", self.theme.name));
            }
            KeyCode::Char('i') => self.show_details = !self.show_details,
            KeyCode::Char('+' | '=') => self.zoom(true),
            KeyCode::Char('-' | '_') => self.zoom(false),
            KeyCode::Char(' ' | 'p') => self.toggle_pause(),
            KeyCode::Char('r') => self.reset(),
            KeyCode::Char('e') => self.export(),
            _ => {}
        }
    }

    fn on_mouse(&mut self, mouse: MouseEvent) {
        if self.mode == Mode::Help {
            if matches!(mouse.kind, MouseEventKind::Down(_)) {
                self.mode = Mode::Normal;
            }
            return;
        }
        let pos = Position::new(mouse.column, mouse.row);
        let in_sidebar = self.areas.sidebar.contains(pos);
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) if self.areas.sidebar_list.contains(pos) => {
                let row = usize::from(mouse.row - self.areas.sidebar_list.y);
                self.select_index(self.list_state.offset() + row / super::ui::SIDEBAR_ITEM_HEIGHT);
            }
            MouseEventKind::ScrollDown if in_sidebar => self.step_selection(true),
            MouseEventKind::ScrollUp if in_sidebar => self.step_selection(false),
            MouseEventKind::ScrollDown if self.areas.details.contains(pos) => {
                self.scroll_details(true, 1);
            }
            MouseEventKind::ScrollUp if self.areas.details.contains(pos) => {
                self.scroll_details(false, 1);
            }
            MouseEventKind::ScrollDown if self.areas.graphs.contains(pos) => self.zoom(false),
            MouseEventKind::ScrollUp if self.areas.graphs.contains(pos) => self.zoom(true),
            _ => {}
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::time::Instant;

    use crossterm::event::KeyEventState;
    use netdev::interface::types::InterfaceType;

    use super::*;
    use crate::net::Counters;
    use crate::net::counters::Snapshot;

    pub(crate) fn iface(name: &str, if_type: InterfaceType, default: bool) -> Interface {
        let mut i = Interface::dummy();
        i.name = name.into();
        i.if_type = if_type;
        i.default = default;
        i
    }

    pub(crate) fn test_app() -> App {
        let interfaces = vec![
            iface("lo0", InterfaceType::Loopback, false),
            iface("en0", InterfaceType::Ethernet, false),
            iface("en1", InterfaceType::Wireless80211, true),
            iface("en2", InterfaceType::Ethernet, false),
        ];
        let mut monitor = Monitor::with_interfaces(interfaces, 64);
        let t0 = Instant::now();
        for step in 0..5u64 {
            let counters = [("lo0", 1), ("en0", 10), ("en1", 1000), ("en2", 100)]
                .into_iter()
                .map(|(n, k)| {
                    (
                        n.to_string(),
                        Counters {
                            rx_bytes: step * k * 1000,
                            tx_bytes: step * k * 10,
                            rx_packets: step * k,
                            ..Counters::default()
                        },
                    )
                })
                .collect();
            monitor.ingest(
                t0 + Duration::from_secs(step),
                &Snapshot {
                    counters,
                    extended: true,
                },
            );
        }
        App::with_monitor(Settings::default(), monitor)
    }

    fn press(app: &mut App, code: KeyCode) {
        app.handle_event(&Event::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }));
    }

    fn names(app: &App) -> Vec<String> {
        app.visible().iter().map(|i| i.name.clone()).collect()
    }

    #[test]
    fn starts_on_default_route_interface() {
        let mut app = test_app();
        assert_eq!(app.selected.as_deref(), Some("en1"));
        assert_eq!(app.sync_selection(), Some(1));
    }

    #[test]
    fn focus_flag_selects_and_reveals_hidden_interface() {
        let monitor = test_app().monitor;
        let settings = Settings {
            focus: vec!["LO0".into()],
            ..Settings::default()
        };
        let app = App::with_monitor(settings, monitor);
        assert_eq!(app.selected.as_deref(), Some("lo0"));
        assert!(app.show_all);
    }

    #[test]
    fn navigation_wraps_and_follows_name_across_sorts() {
        let mut app = test_app();
        assert_eq!(names(&app), ["en0", "en1", "en2"]);
        press(&mut app, KeyCode::Down);
        assert_eq!(app.selected.as_deref(), Some("en2"));
        press(&mut app, KeyCode::Down);
        assert_eq!(app.selected.as_deref(), Some("en0"));
        press(&mut app, KeyCode::Up);
        assert_eq!(app.selected.as_deref(), Some("en2"));

        // Sorting by RX reorders but keeps the selection on the same interface.
        press(&mut app, KeyCode::Char('s'));
        press(&mut app, KeyCode::Char('s'));
        assert_eq!(app.sort, SortKey::Rx);
        assert_eq!(names(&app), ["en1", "en2", "en0"]);
        assert_eq!(app.selected.as_deref(), Some("en2"));
        assert_eq!(app.sync_selection(), Some(1));
    }

    #[test]
    fn number_keys_jump() {
        let mut app = test_app();
        press(&mut app, KeyCode::Char('3'));
        assert_eq!(app.selected.as_deref(), Some("en2"));
        press(&mut app, KeyCode::Char('9'));
        assert_eq!(app.selected.as_deref(), Some("en2"));
    }

    #[test]
    fn filter_mode_narrows_list() {
        let mut app = test_app();
        press(&mut app, KeyCode::Char('/'));
        assert_eq!(app.mode, Mode::Filter);
        press(&mut app, KeyCode::Char('2'));
        assert_eq!(names(&app), ["en2"]);
        app.sync_selection();
        assert_eq!(app.selected.as_deref(), Some("en2"));
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(app.filter, "2");
        // Esc clears an active filter before it would quit.
        press(&mut app, KeyCode::Esc);
        assert!(app.filter.is_empty());
        assert!(app.running);
        press(&mut app, KeyCode::Esc);
        assert!(!app.running);
    }

    #[test]
    fn toggles_all_interfaces() {
        let mut app = test_app();
        press(&mut app, KeyCode::Char('f'));
        assert_eq!(names(&app), ["lo0", "en0", "en1", "en2"]);
    }

    #[test]
    fn zoom_steps_through_levels() {
        let mut app = test_app();
        assert_eq!(app.window, Duration::from_secs(60));
        press(&mut app, KeyCode::Char('+'));
        assert_eq!(app.window, Duration::from_secs(30));
        press(&mut app, KeyCode::Char('-'));
        press(&mut app, KeyCode::Char('-'));
        assert_eq!(app.window, Duration::from_secs(120));
    }

    #[test]
    fn pause_freezes_displayed_data() {
        let mut app = test_app();
        press(&mut app, KeyCode::Char(' '));
        assert!(app.paused());
        let frozen_now = app.data().now();
        app.monitor.ingest(
            Instant::now() + Duration::from_secs(60),
            &Snapshot::default(),
        );
        assert_eq!(app.data().now(), frozen_now);
        press(&mut app, KeyCode::Char(' '));
        assert!(!app.paused());
    }

    #[test]
    fn help_mode_round_trip() {
        let mut app = test_app();
        press(&mut app, KeyCode::Char('?'));
        assert_eq!(app.mode, Mode::Help);
        press(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.running);
    }

    #[test]
    fn cycles_display_options() {
        let mut app = test_app();
        press(&mut app, KeyCode::Char('v'));
        assert_eq!(app.view, GraphView::Combined);
        press(&mut app, KeyCode::Char('m'));
        assert_eq!(app.style, GraphStyle::Block);
        press(&mut app, KeyCode::Char('b'));
        assert_eq!(app.units.mode, crate::units::UnitMode::Bits);
        press(&mut app, KeyCode::Char('t'));
        assert_eq!(app.theme.name, "dracula");
        press(&mut app, KeyCode::Char('r'));
        assert!(app.selected_track().unwrap().history.is_empty());
    }
}
