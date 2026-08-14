//! Shared parsing helpers used by the PN character and name parsers:
//! single-code-point matching, back-track error construction, and
//! recognized-slice extraction.

use winnow::Parser;
use winnow::error::{AddContext, ContextError, ErrMode, Needed, ParserError, StrContext};
use winnow::stream::{Offset, Stream};
use winnow::token::take;

use crate::utf8::utf8_char;
use crate::{Input, RdfResult};

/// Match one code point satisfying `predicate` or back-track.
pub(super) fn char_matching<F>(
    input: &mut Input<'_>,
    predicate: F,
    label: &'static str,
) -> RdfResult<char>
where
    F: Fn(char) -> bool,
{
    let start = input.checkpoint();
    match utf8_char(input) {
        Ok(c) if predicate(c) => Ok(c),
        Ok(_) => {
            input.reset(&start);
            Err(backtrack(input, label))
        }
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&start);
            Err(ErrMode::Incomplete(needed))
        }
        Err(e) => {
            input.reset(&start);
            Err(e)
        }
    }
}

/// Build a labeled Backtrack error at the current position.
pub(super) fn backtrack(input: &Input<'_>, label: &'static str) -> ErrMode<ContextError> {
    let checkpoint = input.checkpoint();
    let err = <ContextError as ParserError<Input<'_>>>::from_input(input);
    ErrMode::Backtrack(err.add_context(input, &checkpoint, StrContext::Label(label)))
}

/// Take the slice between `start` and the current position, leaving
/// the cursor advanced past it.
pub(super) fn slice_since<'a>(
    input: &mut Input<'a>,
    start: <Input<'a> as Stream>::Checkpoint,
) -> RdfResult<&'a [u8]> {
    let end_off = input.offset_from(&start);
    input.reset(&start);
    take::<_, _, ErrMode<ContextError>>(end_off)
        .parse_next(input)
        .map_err(|_| {
            // Should be unreachable — offset was just measured — but
            // degrade gracefully.
            ErrMode::Incomplete(Needed::Unknown)
        })
}

/// Consume the tail of `PN_PREFIX`-style productions: zero or more
/// `(tail_char | '.')`, returning a checkpoint at the last non-dot
/// position so callers can reset to exclude trailing dots.
pub(super) fn consume_tail_with_dots<'a, F>(
    input: &mut Input<'a>,
    tail_set: F,
) -> RdfResult<<Input<'a> as Stream>::Checkpoint>
where
    F: Fn(char) -> bool,
{
    let mut last_ok = input.checkpoint();
    loop {
        let before = input.checkpoint();
        match utf8_char(input) {
            Ok('.') => {
                // Tentative — only part of the prefix if followed by
                // more tail chars.
            }
            Ok(c) if tail_set(c) => {
                last_ok = input.checkpoint();
            }
            Err(ErrMode::Incomplete(needed)) => {
                input.reset(&before);
                return Err(ErrMode::Incomplete(needed));
            }
            Ok(_) | Err(_) => {
                input.reset(&before);
                return Ok(last_ok);
            }
        }
    }
}
