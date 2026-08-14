//! The `PN_PREFIX` and `PN_LOCAL` productions and their head/tail
//! consumption helpers.

use winnow::Parser;
use winnow::error::{ContextError, ErrMode};
use winnow::stream::Stream;
use winnow::token::one_of;

use super::classes::{is_pn_chars, is_pn_chars_base, is_pn_chars_u};
use super::escape::plx;
use super::util::{backtrack, consume_tail_with_dots, slice_since};
use crate::utf8::utf8_char;
use crate::{Input, RdfResult};

/// Match a `PN_PREFIX` and return the recognized byte slice.
///
/// Grammar: `PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?`. Trailing
/// `.` code points are excluded from the returned slice.
pub fn pn_prefix<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    let start = input.checkpoint();

    // Head: PN_CHARS_BASE.
    let first = input.checkpoint();
    match utf8_char(input) {
        Ok(c) if is_pn_chars_base(c) => {}
        Ok(_) => {
            input.reset(&first);
            return Err(backtrack(input, "pn_prefix"));
        }
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&first);
            return Err(ErrMode::Incomplete(needed));
        }
        Err(e) => {
            input.reset(&first);
            return Err(e);
        }
    }

    let last_ok = consume_tail_with_dots(input, is_pn_chars)?;
    input.reset(&last_ok);

    slice_since(input, start)
}

/// Match a `PN_LOCAL` and return the recognized byte slice (PLX
/// escapes are included verbatim).
pub fn pn_local<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    let start = input.checkpoint();

    // Head: PLX | PN_CHARS_U | ':' | [0-9].
    if !consume_pn_local_head(input)? {
        return Err(backtrack(input, "pn_local"));
    }

    // Tail: PLX | PN_CHARS | ':' | '.', with trailing '.' forbidden.
    let mut last_ok = input.checkpoint();
    loop {
        let before = input.checkpoint();
        match try_pn_local_tail_char(input)? {
            TailOutcome::Char { is_dot } => {
                if !is_dot {
                    last_ok = input.checkpoint();
                }
            }
            TailOutcome::End => {
                input.reset(&before);
                break;
            }
        }
    }
    input.reset(&last_ok);

    slice_since(input, start)
}

/// Consume one `(PLX | PN_CHARS_U | ':' | [0-9])` at the head of
/// `PN_LOCAL`. Returns `true` if consumed.
fn consume_pn_local_head(input: &mut Input<'_>) -> RdfResult<bool> {
    let before = input.checkpoint();
    // Try PLX first (may match '%XX' or '\\X').
    match plx(input) {
        Ok(()) => return Ok(true),
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&before);
            return Err(ErrMode::Incomplete(needed));
        }
        Err(_) => input.reset(&before),
    }
    // Try ':' | [0-9].
    if let Ok(b) = one_of::<_, _, ContextError>(b":0123456789").parse_next(input) {
        let _ = b;
        return Ok(true);
    }
    input.reset(&before);
    // Try PN_CHARS_U (needs UTF-8 decode).
    match utf8_char(input) {
        Ok(c) if is_pn_chars_u(c) => Ok(true),
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&before);
            Err(ErrMode::Incomplete(needed))
        }
        Ok(_) | Err(_) => {
            input.reset(&before);
            Ok(false)
        }
    }
}

enum TailOutcome {
    Char { is_dot: bool },
    End,
}

/// Consume one `(PLX | PN_CHARS | ':' | '.')` in the tail of `PN_LOCAL`.
fn try_pn_local_tail_char(input: &mut Input<'_>) -> RdfResult<TailOutcome> {
    let before = input.checkpoint();
    // Try PLX.
    match plx(input) {
        Ok(()) => return Ok(TailOutcome::Char { is_dot: false }),
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&before);
            return Err(ErrMode::Incomplete(needed));
        }
        Err(_) => input.reset(&before),
    }
    // Try ':'.
    if one_of::<_, _, ContextError>(b':').parse_next(input).is_ok() {
        return Ok(TailOutcome::Char { is_dot: false });
    }
    input.reset(&before);
    // Try PN_CHARS | '.'.
    match utf8_char(input) {
        Ok('.') => Ok(TailOutcome::Char { is_dot: true }),
        Ok(c) if is_pn_chars(c) => Ok(TailOutcome::Char { is_dot: false }),
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&before);
            Err(ErrMode::Incomplete(needed))
        }
        Ok(_) | Err(_) => {
            input.reset(&before);
            Ok(TailOutcome::End)
        }
    }
}
