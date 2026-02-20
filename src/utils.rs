// Formats value for verify Kraken orderbook Lvl 2 checksums
pub fn format_value(value: &str) -> String {
    // remove '.', remove leading zeros
    let s = value.replace('.', "");
    s.trim_start_matches('0').to_string()
}