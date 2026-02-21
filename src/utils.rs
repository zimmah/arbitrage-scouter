use std::fs::OpenOptions;
use std::io::Write;

// Formats value for verify Kraken orderbook Lvl 2 checksums
pub fn format_value(value: &str) -> String {
    // remove '.', remove leading zeros
    let s = value.replace('.', "");
    s.trim_start_matches('0').to_string()
}

pub fn debug_log(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug.log")
    {
        let _ = writeln!(file, "{}", msg);
    }
}