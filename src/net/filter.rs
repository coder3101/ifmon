use netdev::{Interface, interface::types::InterfaceType};

/// Whether an interface is worth showing in the default ("physical") view: real adapters plus
/// anything that actually carries addresses, minus loopback, tunnels and never-used adapters.
pub fn is_relevant(iface: &Interface) -> bool {
    let has_ips = !iface.ipv4.is_empty() || !iface.ipv6.is_empty();
    match iface.if_type {
        InterfaceType::Loopback | InterfaceType::Tunnel => false,
        InterfaceType::Ethernet
        | InterfaceType::Wireless80211
        | InterfaceType::GigabitEthernet
        | InterfaceType::FastEthernetT
        | InterfaceType::FastEthernetFx
        | InterfaceType::Wwan
        | InterfaceType::Wwanpp
        | InterfaceType::Wwanpp2 => has_ips || !is_idle(iface),
        _ => has_ips,
    }
}

/// No traffic since boot, e.g. unused Thunderbolt bridge ports on macOS.
fn is_idle(iface: &Interface) -> bool {
    iface
        .stats
        .as_ref()
        .is_some_and(|s| s.rx_bytes == 0 && s.tx_bytes == 0)
}

/// Name shown to users: Windows adapter names are GUIDs, so prefer the friendly name there.
pub fn display_name(iface: &Interface) -> &str {
    if cfg!(windows)
        && let Some(friendly) = iface.friendly_name.as_deref().filter(|f| !f.is_empty())
    {
        return friendly;
    }
    &iface.name
}

/// Case-insensitive match of `query` against the interface name or friendly name.
pub fn matches_query(iface: &Interface, query: &str) -> bool {
    let query = query.to_lowercase();
    iface.name.to_lowercase().contains(&query)
        || iface
            .friendly_name
            .as_deref()
            .is_some_and(|f| f.to_lowercase().contains(&query))
}

/// Exact (case-insensitive) match against the interface name or friendly name.
pub fn matches_exact(iface: &Interface, name: &str) -> bool {
    iface.name.eq_ignore_ascii_case(name)
        || iface
            .friendly_name
            .as_deref()
            .is_some_and(|f| f.eq_ignore_ascii_case(name))
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use netdev::ipnet::{Ipv4Net, Ipv6Net};

    use super::*;

    fn iface(name: &str, if_type: InterfaceType, ipv4: &[&str], ipv6: &[&str]) -> Interface {
        let mut i = Interface::dummy();
        i.name = name.to_string();
        i.if_type = if_type;
        i.ipv4 = ipv4
            .iter()
            .map(|a| Ipv4Net::new(a.parse::<Ipv4Addr>().unwrap(), 24).unwrap())
            .collect();
        i.ipv6 = ipv6
            .iter()
            .map(|a| Ipv6Net::new(a.parse::<Ipv6Addr>().unwrap(), 64).unwrap())
            .collect();
        i
    }

    #[test]
    fn excludes_loopback_even_with_ips() {
        assert!(!is_relevant(&iface(
            "lo",
            InterfaceType::Loopback,
            &["127.0.0.1"],
            &[]
        )));
    }

    #[test]
    fn excludes_tunnel() {
        assert!(!is_relevant(&iface(
            "tun0",
            InterfaceType::Tunnel,
            &[],
            &[]
        )));
    }

    #[test]
    fn virtual_interfaces_need_addresses() {
        assert!(!is_relevant(&iface(
            "vmnet1",
            InterfaceType::ProprietaryVirtual,
            &[],
            &[]
        )));
        assert!(is_relevant(&iface(
            "bridge0",
            InterfaceType::ProprietaryVirtual,
            &["192.168.1.1"],
            &[]
        )));
    }

    #[test]
    fn keeps_physical_without_ips() {
        assert!(is_relevant(&iface(
            "wlan0",
            InterfaceType::Wireless80211,
            &[],
            &[]
        )));
    }

    #[test]
    fn hides_physical_adapters_that_never_carried_traffic() {
        let mut idle = iface("anpi0", InterfaceType::Ethernet, &[], &[]);
        idle.stats = Some(netdev::stats::counters::InterfaceStats {
            rx_bytes: 0,
            tx_bytes: 0,
            timestamp: None,
        });
        assert!(!is_relevant(&idle));

        let mut used = idle.clone();
        used.stats.as_mut().unwrap().rx_bytes = 1;
        assert!(is_relevant(&used));
    }

    #[test]
    fn keeps_unknown_with_any_address() {
        assert!(is_relevant(&iface(
            "eth1",
            InterfaceType::Unknown,
            &["10.0.0.5"],
            &[]
        )));
        assert!(is_relevant(&iface(
            "some0",
            InterfaceType::Unknown,
            &[],
            &["fd00::1"]
        )));
        assert!(!is_relevant(&iface(
            "idle0",
            InterfaceType::Unknown,
            &[],
            &[]
        )));
    }

    #[test]
    fn name_matching() {
        let mut i = iface("en0", InterfaceType::Ethernet, &[], &[]);
        i.friendly_name = Some("Wi-Fi".into());
        assert!(matches_query(&i, "EN"));
        assert!(matches_query(&i, "wi-"));
        assert!(!matches_query(&i, "eth"));
        assert!(matches_exact(&i, "EN0"));
        assert!(matches_exact(&i, "wi-fi"));
        assert!(!matches_exact(&i, "en"));
    }
}
