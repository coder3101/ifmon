use netdev::{interface::types::InterfaceType, Interface};

/// Filter interfaces to show only relevant physical and active interfaces
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
            if matches!(iface.if_type, InterfaceType::ProprietaryVirtual) {
                if iface.ipv4.is_empty() && iface.ipv6.is_empty() {
                    return false;
                }
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
