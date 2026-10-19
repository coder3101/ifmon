use netdev::Interface;
use netdev::interface::state::OperState;
use ratatui::Frame;
use ratatui::layout::{Margin, Rect};
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState};

use super::{label, panel};
use crate::net::display_name;
use crate::theme::Theme;
use crate::tui::app::App;
use crate::units;

const LABEL_WIDTH: usize = 9;

fn row<'a>(theme: &Theme, name: &'a str, value: Vec<Span<'a>>) -> Line<'a> {
    let mut spans = vec![
        label(theme, name),
        Span::raw(" ".repeat(LABEL_WIDTH.saturating_sub(name.len()))),
    ];
    spans.extend(value);
    Line::from(spans)
}

fn text<'a>(theme: &Theme, value: impl Into<String>) -> Vec<Span<'a>> {
    vec![Span::styled(value.into(), Style::new().fg(theme.text))]
}

/// One value per line, labelled on the first; `-` when there are none.
fn multi<'a>(out: &mut Vec<Line<'a>>, theme: &Theme, name: &'a str, values: Vec<Vec<Span<'a>>>) {
    if values.is_empty() {
        out.push(row(
            theme,
            name,
            vec![Span::styled("-", Style::new().fg(theme.dim))],
        ));
    }
    for (i, value) in values.into_iter().enumerate() {
        out.push(row(theme, if i == 0 { name } else { "" }, value));
    }
}

fn flags(iface: &Interface) -> String {
    let mut f = Vec::new();
    if iface.is_up() {
        f.push("UP");
    }
    if iface.is_running() {
        f.push("RUNNING");
    }
    if iface.is_broadcast() {
        f.push("BROADCAST");
    }
    if iface.is_multicast() {
        f.push("MULTICAST");
    }
    if iface.is_point_to_point() {
        f.push("P2P");
    }
    if iface.is_loopback() {
        f.push("LOOPBACK");
    }
    if f.is_empty() {
        "-".into()
    } else {
        f.join(" ")
    }
}

fn lines<'a>(iface: &'a Interface, theme: &Theme) -> Vec<Line<'a>> {
    let mut out = Vec::new();

    let mut name = text(theme, display_name(iface).to_string());
    name[0] = name[0].clone().bold();
    if let Some(friendly) = iface
        .friendly_name
        .as_deref()
        .filter(|f| *f != display_name(iface))
    {
        name.push(Span::styled(
            format!("  ({friendly})"),
            Style::new().fg(theme.dim),
        ));
    }
    if iface.default {
        name.push(Span::styled(
            "  \u{2605} default",
            Style::new().fg(theme.highlight),
        ));
    }
    out.push(row(theme, "Name", name));

    if let Some(desc) = iface.description.as_deref().filter(|d| !d.is_empty()) {
        out.push(row(theme, "Desc", text(theme, desc)));
    }

    let kind = if iface.is_physical() {
        "physical"
    } else {
        "virtual"
    };
    out.push(row(
        theme,
        "Type",
        vec![
            Span::styled(iface.if_type.name(), Style::new().fg(theme.text)),
            Span::styled(
                format!("  {kind} \u{b7} #{}", iface.index),
                Style::new().fg(theme.dim),
            ),
        ],
    ));

    let state_color = match iface.oper_state {
        OperState::Up => theme.ok,
        OperState::Down | OperState::LowerLayerDown | OperState::NotPresent => theme.danger,
        _ => theme.warn,
    };
    out.push(row(
        theme,
        "State",
        vec![
            Span::styled(
                iface.oper_state.as_str().to_uppercase(),
                Style::new().fg(state_color).bold(),
            ),
            Span::styled(format!("  {}", flags(iface)), Style::new().fg(theme.dim)),
        ],
    ));

    let mac = iface
        .mac_addr
        .map_or_else(|| "-".to_string(), |m| m.to_string());
    let mtu = iface.mtu.map_or_else(|| "-".to_string(), |m| m.to_string());
    out.push(row(
        theme,
        "MAC",
        vec![
            Span::styled(mac, Style::new().fg(theme.text)),
            Span::styled("   MTU ", Style::new().fg(theme.dim)),
            Span::styled(mtu, Style::new().fg(theme.text)),
        ],
    ));

    match (iface.receive_speed, iface.transmit_speed) {
        (Some(rx), Some(tx)) if rx == tx => {
            out.push(row(theme, "Link", text(theme, units::link_speed(rx))));
        }
        (rx, tx) if rx.is_some() || tx.is_some() => {
            let fmt = |v: Option<u64>| v.map_or_else(|| "-".to_string(), units::link_speed);
            out.push(row(
                theme,
                "Link",
                text(theme, format!("\u{2193} {}  \u{2191} {}", fmt(rx), fmt(tx))),
            ));
        }
        _ => {}
    }

    if let Some(gw) = &iface.gateway {
        let mut ips: Vec<Vec<Span>> = gw
            .ipv4
            .iter()
            .map(ToString::to_string)
            .chain(gw.ipv6.iter().map(ToString::to_string))
            .map(|ip| text(theme, ip))
            .collect();
        if gw.mac_addr != netdev::MacAddr::zero()
            && let Some(first) = ips.first_mut()
        {
            first.push(Span::styled(
                format!("  {}", gw.mac_addr),
                Style::new().fg(theme.dim),
            ));
        }
        multi(&mut out, theme, "Gateway", ips);
    }

    if !iface.dns_servers.is_empty() {
        let dns = iface.dns_servers.iter().map(|d| text(theme, d.to_string()));
        multi(&mut out, theme, "DNS", dns.collect());
    }

    let v4 = iface.ipv4.iter().map(|a| text(theme, a.to_string()));
    multi(&mut out, theme, "IPv4", v4.collect());
    let v6 = iface.ipv6.iter().map(|a| text(theme, a.to_string()));
    multi(&mut out, theme, "IPv6", v6.collect());

    out
}

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let theme = app.theme.clone();
    let Some(iface) = app.selected_interface() else {
        return;
    };
    let content = lines(iface, &theme);
    let viewport = area.height.saturating_sub(2);
    let max_scroll = (content.len() as u16).saturating_sub(viewport);
    let scroll = app.details_scroll.min(max_scroll);
    let overflow = max_scroll > 0;

    let mut block = panel(&theme, " Interface ", false);
    if overflow {
        block = block.title_bottom(
            Line::from(Span::styled(" J/K scroll ", Style::new().fg(theme.dim))).right_aligned(),
        );
    }
    frame.render_widget(
        Paragraph::new(content).block(block).scroll((scroll, 0)),
        area,
    );

    if overflow {
        let mut state = ScrollbarState::new(usize::from(max_scroll) + 1)
            .viewport_content_length(usize::from(viewport))
            .position(usize::from(scroll));
        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None)
                .style(Style::new().fg(theme.border)),
            area.inner(Margin::new(0, 1)),
            &mut state,
        );
    }
    app.details_scroll = scroll;
}
