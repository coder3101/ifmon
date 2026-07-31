use ratatui::style::Color;

/// Centralized color theme shared across all widgets.
#[derive(Debug, Clone, Copy)]
pub struct Theme {
    pub text: Color,
    pub dim: Color,
    pub border: Color,
    pub accent: Color,
    pub rx: Color,
    pub tx: Color,
    pub ok: Color,
    pub danger: Color,
    pub highlight_fg: Color,
    pub selected_bg: Color,
}

impl Theme {
    pub const fn default() -> Self {
        Self {
            text: Color::White,
            dim: Color::DarkGray,
            border: Color::DarkGray,
            accent: Color::Cyan,
            rx: Color::Cyan,
            tx: Color::Green,
            ok: Color::Green,
            danger: Color::Red,
            highlight_fg: Color::Yellow,
            selected_bg: Color::DarkGray,
        }
    }
}

pub const THEME: Theme = Theme::default();
