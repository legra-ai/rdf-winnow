//! The `PLX` production and its components: `PERCENT` escapes and
//! `PN_LOCAL_ESC` backslash escapes.

use winnow::Parser;
use winnow::combinator::alt;
use winnow::token::one_of;

use crate::{Input, RdfResult};

/// `PLX ::= PERCENT | PN_LOCAL_ESC`.
pub(super) fn plx(input: &mut Input<'_>) -> RdfResult<()> {
    alt((percent, pn_local_esc)).parse_next(input)
}

/// `PERCENT ::= '%' HEX HEX`.
fn percent(input: &mut Input<'_>) -> RdfResult<()> {
    (
        b'%',
        one_of(|b: u8| b.is_ascii_hexdigit()),
        one_of(|b: u8| b.is_ascii_hexdigit()),
    )
        .void()
        .parse_next(input)
}

/// `PN_LOCAL_ESC ::= '\' [_~.!$&'()*+,;=/?#@%-]`.
fn pn_local_esc(input: &mut Input<'_>) -> RdfResult<()> {
    (b'\\', one_of(is_pn_local_esc_byte))
        .void()
        .parse_next(input)
}

fn is_pn_local_esc_byte(b: u8) -> bool {
    matches!(
        b,
        b'_' | b'~'
            | b'.'
            | b'!'
            | b'$'
            | b'&'
            | b'\''
            | b'('
            | b')'
            | b'*'
            | b'+'
            | b','
            | b';'
            | b'='
            | b'/'
            | b'?'
            | b'#'
            | b'@'
            | b'%'
            | b'-'
    )
}
