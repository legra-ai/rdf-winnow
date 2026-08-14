//! Single-code-point parsers for the `PN_CHARS_BASE`, `PN_CHARS_U`,
//! and `PN_CHARS` productions.

use super::classes::{is_pn_chars, is_pn_chars_base, is_pn_chars_u};
use super::util::char_matching;
use crate::{Input, RdfResult};

/// Match one `PN_CHARS_BASE` code point.
pub fn pn_chars_base(input: &mut Input<'_>) -> RdfResult<char> {
    char_matching(input, is_pn_chars_base, "pn_chars_base")
}

/// Match one `PN_CHARS_U` code point (`PN_CHARS_BASE` or `_`).
pub fn pn_chars_u(input: &mut Input<'_>) -> RdfResult<char> {
    char_matching(input, is_pn_chars_u, "pn_chars_u")
}

/// Match one `PN_CHARS` code point.
pub fn pn_chars(input: &mut Input<'_>) -> RdfResult<char> {
    char_matching(input, is_pn_chars, "pn_chars")
}
