/// Scale a byte value down to a human-friendly magnitude with its unit.
fn scale(bytes: f64) -> (f64, &'static str) {
    let kib = 1024.0;
    let mib = 1024.0 * 1024.0;
    let gib = 1024.0 * 1024.0 * 1024.0;

    if bytes < kib {
        (bytes, "B")
    } else if bytes < mib {
        (bytes / kib, "KB")
    } else if bytes < gib {
        (bytes / mib, "MB")
    } else {
        (bytes / gib, "GB")
    }
}

/// Format bytes per second into a human-readable rate.
pub fn format_bytes(bytes: f64) -> String {
    let (v, unit) = scale(bytes);
    format!("{v:.2} {unit}/s")
}

/// Format total bytes into a human-readable quantity.
pub fn format_total_bytes(bytes: u64) -> String {
    let (v, unit) = scale(bytes as f64);
    if unit == "B" {
        format!("{v:.0} {unit}")
    } else {
        format!("{v:.2} {unit}")
    }
}

/// Format an optional value to a string, using "-" when absent.
pub fn format_optional<T: std::fmt::Display>(opt: &Option<T>) -> String {
    match opt {
        Some(n) => n.to_string(),
        None => String::from("-"),
    }
}
