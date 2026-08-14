//! Character-class predicates for the W3C Turtle 1.2 `PN_CHARS_BASE`,
//! `PN_CHARS_U`, and `PN_CHARS` productions.

/// `true` if `c` is in `PN_CHARS_BASE`.
#[must_use]
pub fn is_pn_chars_base(c: char) -> bool {
    let code = c as u32;
    matches!(code,
        0x0041..=0x005A
        | 0x0061..=0x007A
        | 0x00C0..=0x00D6
        | 0x00D8..=0x00F6
        | 0x00F8..=0x02FF
        | 0x0370..=0x037D
        | 0x037F..=0x1FFF
        | 0x200C..=0x200D
        | 0x2070..=0x218F
        | 0x2C00..=0x2FEF
        | 0x3001..=0xD7FF
        | 0xF900..=0xFDCF
        | 0xFDF0..=0xFFFD
        | 0x10000..=0xEFFFF)
}

/// `true` if `c` is in `PN_CHARS_U`.
#[must_use]
pub fn is_pn_chars_u(c: char) -> bool {
    c == '_' || is_pn_chars_base(c)
}

/// `true` if `c` is in `PN_CHARS`.
#[must_use]
pub fn is_pn_chars(c: char) -> bool {
    if is_pn_chars_u(c) {
        return true;
    }
    let code = c as u32;
    matches!(code,
        0x002D
        | 0x0030..=0x0039
        | 0x00B7
        | 0x0300..=0x036F
        | 0x203F..=0x2040)
}
