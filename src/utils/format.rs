/// Format bytes per second into human-readable format
pub fn format_bytes(bytes: f64) -> String {
    if bytes < 1024.0 {
        format!("{:.2} B/s", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2} KB/s", bytes / 1024.0)
    } else if bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} MB/s", bytes / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB/s", bytes / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format total bytes into human-readable format
pub fn format_total_bytes(bytes: u64) -> String {
    let bytes = bytes as f64;
    if bytes < 1024.0 {
        format!("{:.0} B", bytes)
    } else if bytes < 1024.0 * 1024.0 {
        format!("{:.2} KB", bytes / 1024.0)
    } else if bytes < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.2} MB", bytes / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes / (1024.0 * 1024.0 * 1024.0))
    }
}

/// Format optional value to string
pub fn format_optional<T: std::fmt::Display>(opt: &Option<T>) -> String {
    match opt {
        Some(n) => n.to_string(),
        None => String::from("-"),
    }
}
