//! Color themes. Presets can be selected by name and individual colors overridden from config.

use std::str::FromStr;

use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Theme {
    pub name: &'static str,
    pub text: Color,
    pub dim: Color,
    pub border: Color,
    pub border_focus: Color,
    pub accent: Color,
    pub rx: Color,
    pub tx: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
    pub highlight: Color,
    pub selected_bg: Color,
    pub bar_bg: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Theme {
    pub const DEFAULT: Self = Self {
        name: "default",
        text: Color::White,
        dim: Color::DarkGray,
        border: Color::DarkGray,
        border_focus: Color::Cyan,
        accent: Color::Cyan,
        rx: Color::Cyan,
        tx: Color::Green,
        ok: Color::Green,
        warn: Color::Yellow,
        danger: Color::Red,
        highlight: Color::Yellow,
        selected_bg: Color::DarkGray,
        bar_bg: Color::Reset,
    };

    pub const DRACULA: Self = Self {
        name: "dracula",
        text: rgb(0xf8f8f2),
        dim: rgb(0x6272a4),
        border: rgb(0x44475a),
        border_focus: rgb(0xbd93f9),
        accent: rgb(0xbd93f9),
        rx: rgb(0x8be9fd),
        tx: rgb(0xff79c6),
        ok: rgb(0x50fa7b),
        warn: rgb(0xf1fa8c),
        danger: rgb(0xff5555),
        highlight: rgb(0xf1fa8c),
        selected_bg: rgb(0x44475a),
        bar_bg: rgb(0x21222c),
    };

    pub const NORD: Self = Self {
        name: "nord",
        text: rgb(0xeceff4),
        dim: rgb(0x616e88),
        border: rgb(0x434c5e),
        border_focus: rgb(0x88c0d0),
        accent: rgb(0x88c0d0),
        rx: rgb(0x81a1c1),
        tx: rgb(0xa3be8c),
        ok: rgb(0xa3be8c),
        warn: rgb(0xebcb8b),
        danger: rgb(0xbf616a),
        highlight: rgb(0xebcb8b),
        selected_bg: rgb(0x3b4252),
        bar_bg: rgb(0x2e3440),
    };

    pub const GRUVBOX: Self = Self {
        name: "gruvbox",
        text: rgb(0xebdbb2),
        dim: rgb(0x928374),
        border: rgb(0x504945),
        border_focus: rgb(0xfabd2f),
        accent: rgb(0xfabd2f),
        rx: rgb(0x83a598),
        tx: rgb(0xb8bb26),
        ok: rgb(0xb8bb26),
        warn: rgb(0xfe8019),
        danger: rgb(0xfb4934),
        highlight: rgb(0xfabd2f),
        selected_bg: rgb(0x3c3836),
        bar_bg: rgb(0x282828),
    };

    pub const CATPPUCCIN: Self = Self {
        name: "catppuccin",
        text: rgb(0xcdd6f4),
        dim: rgb(0x7f849c),
        border: rgb(0x45475a),
        border_focus: rgb(0xcba6f7),
        accent: rgb(0xcba6f7),
        rx: rgb(0x89b4fa),
        tx: rgb(0xa6e3a1),
        ok: rgb(0xa6e3a1),
        warn: rgb(0xf9e2af),
        danger: rgb(0xf38ba8),
        highlight: rgb(0xf9e2af),
        selected_bg: rgb(0x313244),
        bar_bg: rgb(0x181825),
    };

    pub const TOKYO_NIGHT: Self = Self {
        name: "tokyonight",
        text: rgb(0xc0caf5),
        dim: rgb(0x565f89),
        border: rgb(0x3b4261),
        border_focus: rgb(0x7aa2f7),
        accent: rgb(0x7aa2f7),
        rx: rgb(0x7dcfff),
        tx: rgb(0x9ece6a),
        ok: rgb(0x9ece6a),
        warn: rgb(0xe0af68),
        danger: rgb(0xf7768e),
        highlight: rgb(0xe0af68),
        selected_bg: rgb(0x292e42),
        bar_bg: rgb(0x16161e),
    };

    pub const SOLARIZED: Self = Self {
        name: "solarized",
        text: rgb(0x93a1a1),
        dim: rgb(0x586e75),
        border: rgb(0x073642),
        border_focus: rgb(0x268bd2),
        accent: rgb(0x268bd2),
        rx: rgb(0x2aa198),
        tx: rgb(0x859900),
        ok: rgb(0x859900),
        warn: rgb(0xb58900),
        danger: rgb(0xdc322f),
        highlight: rgb(0xb58900),
        selected_bg: rgb(0x073642),
        bar_bg: rgb(0x002b36),
    };

    pub const MONO: Self = Self {
        name: "mono",
        text: Color::Reset,
        dim: Color::Reset,
        border: Color::Reset,
        border_focus: Color::Reset,
        accent: Color::Reset,
        rx: Color::Reset,
        tx: Color::Reset,
        ok: Color::Reset,
        warn: Color::Reset,
        danger: Color::Reset,
        highlight: Color::Reset,
        selected_bg: Color::Reset,
        bar_bg: Color::Reset,
    };

    pub const PRESETS: [Self; 8] = [
        Self::DEFAULT,
        Self::DRACULA,
        Self::NORD,
        Self::GRUVBOX,
        Self::CATPPUCCIN,
        Self::TOKYO_NIGHT,
        Self::SOLARIZED,
        Self::MONO,
    ];

    pub fn by_name(name: &str) -> Option<Self> {
        Self::PRESETS
            .into_iter()
            .find(|t| t.name.eq_ignore_ascii_case(name))
    }

    pub fn names() -> impl Iterator<Item = &'static str> {
        Self::PRESETS.iter().map(|t| t.name)
    }

    /// The preset after this one, wrapping around.
    pub fn next_preset(&self) -> Self {
        let idx = Self::PRESETS
            .iter()
            .position(|t| t.name == self.name)
            .map_or(0, |i| (i + 1) % Self::PRESETS.len());
        Self::PRESETS[idx].clone()
    }

    pub fn is_mono(&self) -> bool {
        self.name == Self::MONO.name
    }

    /// Bold "pill" style with `bg` as the background, readable in every theme.
    pub fn badge(&self, bg: Color) -> Style {
        if self.is_mono() {
            return Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD);
        }
        let fg = match self.bar_bg {
            Color::Reset => Color::Black,
            c => c,
        };
        Style::new().fg(fg).bg(bg).add_modifier(Modifier::BOLD)
    }

    /// Style for the selected list row.
    pub fn selection(&self) -> Style {
        if self.is_mono() {
            return Style::new().add_modifier(Modifier::REVERSED);
        }
        Style::new()
            .fg(self.highlight)
            .bg(self.selected_bg)
            .add_modifier(Modifier::BOLD)
    }

    /// Apply user color overrides on top of this theme.
    pub fn with_overrides(mut self, o: &ColorOverrides) -> Result<Self, String> {
        let slots: [(&Option<String>, &mut Color); 13] = [
            (&o.text, &mut self.text),
            (&o.dim, &mut self.dim),
            (&o.border, &mut self.border),
            (&o.border_focus, &mut self.border_focus),
            (&o.accent, &mut self.accent),
            (&o.rx, &mut self.rx),
            (&o.tx, &mut self.tx),
            (&o.ok, &mut self.ok),
            (&o.warn, &mut self.warn),
            (&o.danger, &mut self.danger),
            (&o.highlight, &mut self.highlight),
            (&o.selected_bg, &mut self.selected_bg),
            (&o.bar_bg, &mut self.bar_bg),
        ];
        for (value, slot) in slots {
            if let Some(v) = value {
                *slot = Color::from_str(v).map_err(|_| {
                    format!("invalid color '{v}' (use a name like 'cyan' or '#rrggbb')")
                })?;
            }
        }
        Ok(self)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Optional per-color overrides, as written in the `[colors]` config table.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct ColorOverrides {
    pub text: Option<String>,
    pub dim: Option<String>,
    pub border: Option<String>,
    pub border_focus: Option<String>,
    pub accent: Option<String>,
    pub rx: Option<String>,
    pub tx: Option<String>,
    pub ok: Option<String>,
    pub warn: Option<String>,
    pub danger: Option<String>,
    pub highlight: Option<String>,
    pub selected_bg: Option<String>,
    pub bar_bg: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_up_presets_case_insensitively() {
        assert_eq!(Theme::by_name("Nord").unwrap().name, "nord");
        assert!(Theme::by_name("nope").is_none());
        assert_eq!(Theme::names().count(), Theme::PRESETS.len());
    }

    #[test]
    fn cycles_through_all_presets() {
        let mut t = Theme::DEFAULT;
        for _ in 0..Theme::PRESETS.len() {
            t = t.next_preset();
        }
        assert_eq!(t.name, Theme::DEFAULT.name);
    }

    #[test]
    fn applies_overrides() {
        let o = ColorOverrides {
            rx: Some("#ff0000".into()),
            tx: Some("magenta".into()),
            ..ColorOverrides::default()
        };
        let t = Theme::DEFAULT.with_overrides(&o).unwrap();
        assert_eq!(t.rx, Color::Rgb(255, 0, 0));
        assert_eq!(t.tx, Color::Magenta);
        assert_eq!(t.ok, Theme::DEFAULT.ok);

        let bad = ColorOverrides {
            ok: Some("not-a-color".into()),
            ..ColorOverrides::default()
        };
        assert!(Theme::DEFAULT.with_overrides(&bad).is_err());
    }
}
