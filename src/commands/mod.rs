pub mod config;
pub mod list;
pub mod watch;

use std::io;

use color_eyre::eyre::{Result, bail};
use netdev::Interface;

use crate::net::{display_name, is_relevant, matches_exact};

/// Pick the interfaces a non-interactive command should report on: the explicitly requested
/// ones (all must exist), otherwise everything passing the physical/all filter.
pub fn select<'a>(
    interfaces: &'a [Interface],
    focus: &[String],
    show_all: bool,
) -> Result<Vec<&'a Interface>> {
    if focus.is_empty() {
        return Ok(interfaces
            .iter()
            .filter(|i| show_all || is_relevant(i))
            .collect());
    }
    let mut out = Vec::new();
    for name in focus {
        match interfaces.iter().find(|i| matches_exact(i, name)) {
            Some(iface) if !out.iter().any(|o: &&Interface| o.name == iface.name) => {
                out.push(iface);
            }
            Some(_) => {}
            None => {
                let available: Vec<&str> = interfaces.iter().map(display_name).collect();
                bail!(
                    "no interface named '{name}' (available: {})",
                    available.join(", ")
                );
            }
        }
    }
    Ok(out)
}

/// Treat a closed stdout (e.g. `ifmon watch | head`) as a normal exit.
pub fn quiet_broken_pipe(result: io::Result<()>) -> Result<()> {
    match result {
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => Ok(()),
        other => Ok(other?),
    }
}

#[cfg(test)]
mod tests {
    use netdev::interface::types::InterfaceType;

    use super::*;

    fn iface(name: &str, if_type: InterfaceType) -> Interface {
        let mut i = Interface::dummy();
        i.name = name.into();
        i.if_type = if_type;
        i
    }

    #[test]
    fn selects_by_filter_or_name() {
        let all = vec![
            iface("lo", InterfaceType::Loopback),
            iface("eth0", InterfaceType::Ethernet),
        ];
        let names = |v: Vec<&Interface>| v.iter().map(|i| i.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(select(&all, &[], false).unwrap()), ["eth0"]);
        assert_eq!(names(select(&all, &[], true).unwrap()), ["lo", "eth0"]);
        let focus = ["LO".to_string(), "lo".to_string()];
        assert_eq!(names(select(&all, &focus, false).unwrap()), ["lo"]);
        assert!(select(&all, &["wlan9".into()], false).is_err());
    }
}
