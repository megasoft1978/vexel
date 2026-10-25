//! `#rrggbb` and short `#rgb` colour parsing shared by the element tools.

/// `#rgb` expanded to `#rrggbb`; any other text is returned unchanged.
pub(super) fn expand_hex(value: &str) -> String {
    let bytes = value.as_bytes();
    if bytes.len() == 4 && bytes[0] == b'#' && bytes[1..].iter().all(u8::is_ascii_hexdigit) {
        let mut long = String::with_capacity(7);
        long.push('#');
        for digit in value[1..].chars() {
            long.push(digit);
            long.push(digit);
        }
        return long;
    }
    value.to_owned()
}

pub(super) fn parse_color(value: &str) -> Result<u32, &'static str> {
    let value = expand_hex(value);
    if value.len() != 7
        || !value.starts_with('#')
        || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return Err("invalid preview fill color");
    }
    u32::from_str_radix(&value[1..], 16).map_err(|_| "invalid preview fill color")
}
