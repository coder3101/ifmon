//! Cumulative per-interface traffic counters read straight from the OS.
//!
//! `netdev` only exposes byte counters (and 32-bit ones on macOS that wrap at 4 GiB), so the
//! common platforms get a native reader that also yields packets, errors and drops:
//!
//! * Linux: a single read of `/proc/net/dev`
//! * macOS: the `ifmib` sysctl, which returns exact 64-bit `if_data64` counters
//! * Windows: `GetIfEntry2`
//!
//! Every other platform falls back to `netdev`'s byte counters.

use std::collections::HashMap;

use netdev::Interface;
use serde::Serialize;

/// Cumulative counters for one interface, as reported by the OS.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct Counters {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_drops: u64,
    pub tx_drops: u64,
}

impl Counters {
    /// Counter increase since `prev`, tolerating 32-bit wraparound and counter resets.
    pub fn delta_since(&self, prev: &Self) -> Self {
        Self {
            rx_bytes: counter_delta(prev.rx_bytes, self.rx_bytes),
            tx_bytes: counter_delta(prev.tx_bytes, self.tx_bytes),
            rx_packets: counter_delta(prev.rx_packets, self.rx_packets),
            tx_packets: counter_delta(prev.tx_packets, self.tx_packets),
            rx_errors: counter_delta(prev.rx_errors, self.rx_errors),
            tx_errors: counter_delta(prev.tx_errors, self.tx_errors),
            rx_drops: counter_delta(prev.rx_drops, self.rx_drops),
            tx_drops: counter_delta(prev.tx_drops, self.tx_drops),
        }
    }

    pub fn saturating_add(&self, other: &Self) -> Self {
        Self {
            rx_bytes: self.rx_bytes.saturating_add(other.rx_bytes),
            tx_bytes: self.tx_bytes.saturating_add(other.tx_bytes),
            rx_packets: self.rx_packets.saturating_add(other.rx_packets),
            tx_packets: self.tx_packets.saturating_add(other.tx_packets),
            rx_errors: self.rx_errors.saturating_add(other.rx_errors),
            tx_errors: self.tx_errors.saturating_add(other.tx_errors),
            rx_drops: self.rx_drops.saturating_add(other.rx_drops),
            tx_drops: self.tx_drops.saturating_add(other.tx_drops),
        }
    }
}

fn counter_delta(prev: u64, cur: u64) -> u64 {
    const U32_MAX: u64 = u32::MAX as u64;
    if cur >= prev {
        cur - prev
    } else if prev <= U32_MAX && prev > U32_MAX / 2 {
        // A 32-bit counter that wrapped around.
        (U32_MAX - prev) + cur + 1
    } else {
        // The counter was reset (e.g. the interface was re-created).
        cur
    }
}

/// Counters for every interface at one point in time, keyed by interface name.
#[derive(Debug, Clone, Default)]
pub struct Snapshot {
    pub counters: HashMap<String, Counters>,
    /// Whether packet, error and drop counters are populated on this platform.
    pub extended: bool,
}

/// Read counters for all `interfaces`.
pub fn read(interfaces: &mut [Interface]) -> Snapshot {
    platform::read(interfaces).unwrap_or_else(|| fallback(interfaces))
}

/// Byte-only counters via `netdev`, used where no native reader exists or it failed.
fn fallback(interfaces: &mut [Interface]) -> Snapshot {
    let counters = interfaces
        .iter_mut()
        .filter_map(|iface| {
            let _ = iface.update_stats();
            let stats = iface.stats.as_ref()?;
            Some((
                iface.name.clone(),
                Counters {
                    rx_bytes: stats.rx_bytes,
                    tx_bytes: stats.tx_bytes,
                    ..Counters::default()
                },
            ))
        })
        .collect();
    Snapshot {
        counters,
        extended: false,
    }
}

/// Parse the contents of Linux's `/proc/net/dev`.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
fn parse_proc_net_dev(contents: &str) -> HashMap<String, Counters> {
    contents
        .lines()
        .skip(2)
        .filter_map(|line| {
            let (name, rest) = line.rsplit_once(':')?;
            let f: Vec<u64> = rest
                .split_whitespace()
                .map(|v| v.parse().unwrap_or(0))
                .collect();
            if f.len() < 16 {
                return None;
            }
            Some((
                name.trim().to_string(),
                Counters {
                    rx_bytes: f[0],
                    rx_packets: f[1],
                    rx_errors: f[2],
                    rx_drops: f[3],
                    tx_bytes: f[8],
                    tx_packets: f[9],
                    tx_errors: f[10],
                    tx_drops: f[11],
                },
            ))
        })
        .collect()
}

#[cfg(target_os = "linux")]
mod platform {
    use super::{Interface, Snapshot, parse_proc_net_dev};

    pub fn read(_interfaces: &mut [Interface]) -> Option<Snapshot> {
        let contents = std::fs::read_to_string("/proc/net/dev").ok()?;
        Some(Snapshot {
            counters: parse_proc_net_dev(&contents),
            extended: true,
        })
    }
}

#[cfg(target_os = "macos")]
mod platform {
    use std::mem::{size_of, zeroed};
    use std::ptr;

    use super::{Counters, Interface, Snapshot};

    pub fn read(interfaces: &mut [Interface]) -> Option<Snapshot> {
        let counters: std::collections::HashMap<_, _> = interfaces
            .iter()
            .filter_map(|iface| Some((iface.name.clone(), read_ifmib(iface.index)?)))
            .collect();
        if counters.is_empty() && !interfaces.is_empty() {
            return None;
        }
        Some(Snapshot {
            counters,
            extended: true,
        })
    }

    /// Per-interface `net.link.generic.ifdata.<index>.general`. Unlike `getifaddrs` and
    /// `NET_RT_IFLIST2`, which hand unprivileged processes byte counts truncated to 32 bits and
    /// rounded to 1 KiB, this returns the exact 64-bit counters that `netstat -ib` shows.
    fn read_ifmib(index: u32) -> Option<Counters> {
        let mut mib = [
            libc::CTL_NET,
            libc::PF_LINK,
            libc::NETLINK_GENERIC,
            libc::IFMIB_IFDATA,
            libc::c_int::try_from(index).ok()?,
            libc::IFDATA_GENERAL,
        ];
        // SAFETY: ifmibdata is plain old data; sysctl writes at most `len` bytes into it.
        let mut data: libc::ifmibdata = unsafe { zeroed() };
        let mut len = size_of::<libc::ifmibdata>();
        let rc = unsafe {
            libc::sysctl(
                mib.as_mut_ptr(),
                mib.len() as libc::c_uint,
                (&raw mut data).cast(),
                &raw mut len,
                ptr::null_mut(),
                0,
            )
        };
        if rc != 0 {
            return None;
        }
        let d = data.ifmd_data;
        Some(Counters {
            rx_bytes: d.ifi_ibytes,
            tx_bytes: d.ifi_obytes,
            rx_packets: d.ifi_ipackets,
            tx_packets: d.ifi_opackets,
            rx_errors: d.ifi_ierrors,
            tx_errors: d.ifi_oerrors,
            rx_drops: d.ifi_iqdrops,
            tx_drops: u64::from(data.ifmd_snd_drops),
        })
    }
}

#[cfg(windows)]
mod platform {
    use std::collections::HashMap;
    use std::mem::zeroed;

    use windows_sys::Win32::NetworkManagement::IpHelper::{GetIfEntry2, MIB_IF_ROW2};

    use super::{Counters, Interface, Snapshot};

    pub fn read(interfaces: &mut [Interface]) -> Option<Snapshot> {
        let counters = interfaces
            .iter()
            .filter_map(|iface| {
                // SAFETY: MIB_IF_ROW2 is plain old data; GetIfEntry2 fills it for the given index.
                let mut row: MIB_IF_ROW2 = unsafe { zeroed() };
                row.InterfaceIndex = iface.index;
                if unsafe { GetIfEntry2(&raw mut row) } != 0 {
                    return None;
                }
                Some((
                    iface.name.clone(),
                    Counters {
                        rx_bytes: row.InOctets,
                        tx_bytes: row.OutOctets,
                        rx_packets: row.InUcastPkts.saturating_add(row.InNUcastPkts),
                        tx_packets: row.OutUcastPkts.saturating_add(row.OutNUcastPkts),
                        rx_errors: row.InErrors,
                        tx_errors: row.OutErrors,
                        rx_drops: row.InDiscards,
                        tx_drops: row.OutDiscards,
                    },
                ))
            })
            .collect::<HashMap<_, _>>();
        if counters.is_empty() && !interfaces.is_empty() {
            return None;
        }
        Some(Snapshot {
            counters,
            extended: true,
        })
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
mod platform {
    use super::{Interface, Snapshot};

    pub fn read(_interfaces: &mut [Interface]) -> Option<Snapshot> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_is_plain_difference() {
        assert_eq!(counter_delta(100, 250), 150);
        assert_eq!(counter_delta(5, 5), 0);
    }

    #[test]
    fn delta_handles_32bit_wrap() {
        let prev = u64::from(u32::MAX) - 9;
        assert_eq!(counter_delta(prev, 5), 15);
    }

    #[test]
    fn delta_treats_small_decrease_as_reset() {
        assert_eq!(counter_delta(1_000, 40), 40);
        assert_eq!(counter_delta(10_000_000_000, 7), 7);
    }

    #[test]
    fn delta_and_add_cover_every_field() {
        let a = Counters {
            rx_bytes: 1,
            tx_bytes: 2,
            rx_packets: 3,
            tx_packets: 4,
            rx_errors: 5,
            tx_errors: 6,
            rx_drops: 7,
            tx_drops: 8,
        };
        let doubled = a.saturating_add(&a);
        assert_eq!(doubled.delta_since(&a), a);
    }

    #[test]
    fn parses_proc_net_dev() {
        let sample = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
    lo:  123456     789    0    0    0     0          0         0   123456     789    0    0    0     0       0          0
  eth0: 9876543   12345    2    3    0     0          0        10  5432100    6789    4    5    0     0       0          0
garbage line
";
        let parsed = parse_proc_net_dev(sample);
        assert_eq!(parsed.len(), 2);
        let eth0 = parsed["eth0"];
        assert_eq!(eth0.rx_bytes, 9_876_543);
        assert_eq!(eth0.rx_packets, 12_345);
        assert_eq!(eth0.rx_errors, 2);
        assert_eq!(eth0.rx_drops, 3);
        assert_eq!(eth0.tx_bytes, 5_432_100);
        assert_eq!(eth0.tx_packets, 6_789);
        assert_eq!(eth0.tx_errors, 4);
        assert_eq!(eth0.tx_drops, 5);
    }
}
