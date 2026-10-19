use std::io::{self, Write};

use color_eyre::eyre::Result;
use netdev::Interface;
use serde::Serialize;
use unicode_width::UnicodeWidthStr;

use crate::cli::ListArgs;
use crate::net::{Counters, counters, display_name};
use crate::settings::Settings;

#[derive(Serialize)]
struct Gateway {
    mac: String,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
}

#[derive(Serialize)]
struct Record<'a> {
    name: &'a str,
    friendly_name: Option<&'a str>,
    description: Option<&'a str>,
    index: u32,
    #[serde(rename = "type")]
    if_type: String,
    state: &'static str,
    up: bool,
    running: bool,
    physical: bool,
    default_route: bool,
    mac: Option<String>,
    mtu: Option<u32>,
    link_speed_rx_bps: Option<u64>,
    link_speed_tx_bps: Option<u64>,
    ipv4: Vec<String>,
    ipv6: Vec<String>,
    gateway: Option<Gateway>,
    dns_servers: Vec<String>,
    counters: Option<Counters>,
}

impl<'a> Record<'a> {
    fn new(iface: &'a Interface, counters: Option<Counters>) -> Self {
        Self {
            name: &iface.name,
            friendly_name: iface.friendly_name.as_deref(),
            description: iface.description.as_deref(),
            index: iface.index,
            if_type: iface.if_type.name(),
            state: iface.oper_state.as_str(),
            up: iface.is_up(),
            running: iface.is_running(),
            physical: iface.is_physical(),
            default_route: iface.default,
            mac: iface.mac_addr.map(|m| m.to_string()),
            mtu: iface.mtu,
            link_speed_rx_bps: iface.receive_speed,
            link_speed_tx_bps: iface.transmit_speed,
            ipv4: iface.ipv4.iter().map(ToString::to_string).collect(),
            ipv6: iface.ipv6.iter().map(ToString::to_string).collect(),
            gateway: iface.gateway.as_ref().map(|g| Gateway {
                mac: g.mac_addr.to_string(),
                ipv4: g.ipv4.iter().map(ToString::to_string).collect(),
                ipv6: g.ipv6.iter().map(ToString::to_string).collect(),
            }),
            dns_servers: iface.dns_servers.iter().map(ToString::to_string).collect(),
            counters,
        }
    }
}

pub fn run(settings: &Settings, args: &ListArgs) -> Result<()> {
    let mut interfaces = netdev::get_interfaces();
    let snapshot = counters::read(&mut interfaces);
    let selected = super::select(&interfaces, &settings.focus, settings.show_all)?;

    let mut out = io::stdout().lock();
    if args.json {
        let records: Vec<Record> = selected
            .iter()
            .map(|i| Record::new(i, snapshot.counters.get(&i.name).copied()))
            .collect();
        return super::quiet_broken_pipe(
            serde_json::to_writer_pretty(&mut out, &records)
                .map_err(io::Error::from)
                .and_then(|()| writeln!(out)),
        );
    }

    let header = [
        "", "NAME", "TYPE", "STATE", "MAC", "MTU", "ADDRESS", "RX", "TX",
    ];
    let rows: Vec<[String; 9]> = selected
        .iter()
        .map(|iface| {
            let c = snapshot.counters.get(&iface.name);
            let addr = iface
                .ipv4
                .first()
                .map(ToString::to_string)
                .or_else(|| iface.ipv6.first().map(ToString::to_string))
                .unwrap_or_else(|| "-".into());
            let extra = (iface.ipv4.len() + iface.ipv6.len()).saturating_sub(1);
            let addr = if extra > 0 {
                format!("{addr} (+{extra})")
            } else {
                addr
            };
            [
                if iface.default { "*" } else { "" }.into(),
                display_name(iface).into(),
                iface.if_type.name(),
                iface.oper_state.as_str().into(),
                iface.mac_addr.map_or_else(|| "-".into(), |m| m.to_string()),
                iface.mtu.map_or_else(|| "-".into(), |m| m.to_string()),
                addr,
                c.map_or_else(|| "-".into(), |c| settings.units.size(c.rx_bytes)),
                c.map_or_else(|| "-".into(), |c| settings.units.size(c.tx_bytes)),
            ]
        })
        .collect();

    let mut widths = header.map(UnicodeWidthStr::width);
    for row in &rows {
        for (w, cell) in widths.iter_mut().zip(row) {
            *w = (*w).max(cell.width());
        }
    }

    let render = |cells: &[&str]| -> String {
        let mut line = String::new();
        for (i, (cell, w)) in cells.iter().zip(widths).enumerate() {
            let pad = w.saturating_sub(cell.width());
            // Right-align the counter columns.
            if i >= 7 {
                line.push_str(&" ".repeat(pad));
                line.push_str(cell);
            } else {
                line.push_str(cell);
                line.push_str(&" ".repeat(pad));
            }
            line.push_str("  ");
        }
        line.trim_end().to_string()
    };

    let result = (|| {
        writeln!(out, "{}", render(&header))?;
        for row in &rows {
            let cells: Vec<&str> = row.iter().map(String::as_str).collect();
            writeln!(out, "{}", render(&cells))?;
        }
        if selected.iter().any(|i| i.default) {
            writeln!(out, "\n* default route")?;
        }
        Ok(())
    })();
    super::quiet_broken_pipe(result)
}
