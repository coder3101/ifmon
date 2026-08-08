use netdev::{interface::types::InterfaceType, Interface};

/// Filter interfaces to show only relevant physical and active interfaces.
pub fn filter_interfaces(interfaces: &[Interface]) -> Vec<Interface> {
    interfaces
        .iter()
        .filter(|iface| {
            // Exclude loopback interfaces
            if matches!(iface.if_type, InterfaceType::Loopback) {
                return false;
            }

            // Exclude tunnel interfaces
            if matches!(iface.if_type, InterfaceType::Tunnel) {
                return false;
            }

            // Exclude proprietary virtual interfaces without IPs
            if matches!(iface.if_type, InterfaceType::ProprietaryVirtual)
                && iface.ipv4.is_empty()
                && iface.ipv6.is_empty()
            {
                return false;
            }

            // Include physical network interfaces
            if matches!(
                iface.if_type,
                InterfaceType::Ethernet
                    | InterfaceType::Wireless80211
                    | InterfaceType::GigabitEthernet
                    | InterfaceType::FastEthernetT
                    | InterfaceType::FastEthernetFx
                    | InterfaceType::Wwan
                    | InterfaceType::Wwanpp
                    | InterfaceType::Wwanpp2
            ) {
                return true;
            }

            // Include any interface that has IP addresses (is actually in use)
            if !iface.ipv4.is_empty() || !iface.ipv6.is_empty() {
                return true;
            }

            // Exclude everything else
            false
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use std::net::{Ipv4Addr, Ipv6Addr};

    use netdev::ipnet::{Ipv4Net, Ipv6Net};

    use super::*;

    fn iface(
        name: &str,
        if_type: InterfaceType,
        ipv4: Vec<Ipv4Net>,
        ipv6: Vec<Ipv6Net>,
    ) -> Interface {
        let mut i = Interface::dummy();
        i.name = name.to_string();
        i.if_type = if_type;
        i.ipv4 = ipv4;
        i.ipv6 = ipv6;
        i
    }

    fn v4(addr: &str) -> Ipv4Net {
        Ipv4Net::new(addr.parse::<Ipv4Addr>().unwrap(), 24).unwrap()
    }

    fn v6(addr: &str) -> Ipv6Net {
        Ipv6Net::new(addr.parse::<Ipv6Addr>().unwrap(), 64).unwrap()
    }

    #[test]
    fn excludes_loopback() {
        let interfaces = vec![
            iface("lo", InterfaceType::Loopback, vec![v4("127.0.0.1")], vec![]),
            iface("eth0", InterfaceType::Ethernet, vec![], vec![]),
        ];
        let filtered = filter_interfaces(&interfaces);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "eth0");
    }

    #[test]
    fn excludes_tunnel() {
        let interfaces = vec![iface("tun0", InterfaceType::Tunnel, vec![], vec![])];
        assert!(filter_interfaces(&interfaces).is_empty());
    }

    #[test]
    fn excludes_virtual_without_ips() {
        let interfaces = vec![iface(
            "vmnet1",
            InterfaceType::ProprietaryVirtual,
            vec![],
            vec![],
        )];
        assert!(filter_interfaces(&interfaces).is_empty());
    }

    #[test]
    fn keeps_virtual_with_ips() {
        let interfaces = vec![iface(
            "bridge0",
            InterfaceType::ProprietaryVirtual,
            vec![v4("192.168.1.1")],
            vec![],
        )];
        assert_eq!(filter_interfaces(&interfaces).len(), 1);
    }

    #[test]
    fn keeps_interface_with_only_ipv6() {
        let interfaces = vec![iface(
            "some0",
            InterfaceType::Unknown,
            vec![],
            vec![v6("fd00::1")],
        )];
        assert_eq!(filter_interfaces(&interfaces).len(), 1);
    }

    #[test]
    fn keeps_physical_without_ips() {
        let interfaces = vec![iface("wlan0", InterfaceType::Wireless80211, vec![], vec![])];
        assert_eq!(filter_interfaces(&interfaces).len(), 1);
    }

    #[test]
    fn keeps_unknown_with_ips() {
        let interfaces = vec![iface(
            "eth1",
            InterfaceType::Unknown,
            vec![v4("10.0.0.5")],
            vec![],
        )];
        assert_eq!(filter_interfaces(&interfaces).len(), 1);
    }
}
