//! Byte-level UTF-8 code-point helper.
//!
//! The RDF text primitives operate on `Partial<&[u8]>` because that is
//! the type an async chunk-feeder can append to without risking a
//! code-point split across a UTF-8 boundary. For grammar productions
//! that depend on Unicode code-point ranges (notably `PN_CHARS_BASE`
//! in Turtle's prefixed-name productions), this helper reads one
//! complete UTF-8 code point from the stream and returns the decoded
//! `char`. On a truncated sequence (buffer ends mid-code-point) it
//! returns `ErrMode::Incomplete` so the feeder can extend the buffer
//! and retry.

use winnow::Parser;
use winnow::error::{AddContext, ContextError, ErrMode, Needed, ParserError, StrContext};
use winnow::stream::Stream;
use winnow::token::take;

use crate::{Input, RdfResult};

/// Decode and consume one UTF-8 code point.
///
/// # Errors
///
/// - `ErrMode::Incomplete(Needed)` if the input ends before a full code point
///   is available.
/// - `ErrMode::Backtrack` tagged `"utf8_char"` if the leading byte is not a
///   valid UTF-8 start byte, or if the decoded bytes are not valid UTF-8.
pub fn utf8_char(input: &mut Input<'_>) -> RdfResult<char> {
    let bytes: &[u8] = &*input;
    if bytes.is_empty() {
        return Err(ErrMode::Incomplete(Needed::new(1)));
    }
    let len = match bytes[0] {
        0x00..=0x7F => 1,
        0xC2..=0xDF => 2,
        0xE0..=0xEF => 3,
        0xF0..=0xF4 => 4,
        _ => return Err(backtrack_utf8_error(input)),
    };
    if bytes.len() < len {
        return Err(ErrMode::Incomplete(Needed::new(len - bytes.len())));
    }
    let slice: &[u8] = take(len).parse_next(input)?;
    let s = std::str::from_utf8(slice).map_err(|_| backtrack_utf8_error(input))?;
    Ok(s.chars()
        .next()
        .expect("validated UTF-8 slice is non-empty"))
}

fn backtrack_utf8_error(input: &Input<'_>) -> ErrMode<ContextError> {
    let checkpoint = input.checkpoint();
    let err = <ContextError as ParserError<Input<'_>>>::from_input(input);
    ErrMode::Backtrack(err.add_context(input, &checkpoint, StrContext::Label("utf8_char")))
}

#[cfg(test)]
mod tests {
    use winnow::Partial;

    use super::*;

    #[test]
    fn decodes_ascii() {
        let mut inp: Input<'_> = Partial::new(b"A");
        assert_eq!(utf8_char(&mut inp).unwrap(), 'A');
    }

    #[test]
    fn decodes_two_byte_sequence() {
        // U+00E9 (é) -> C3 A9
        let mut inp: Input<'_> = Partial::new(&[0xC3, 0xA9]);
        assert_eq!(utf8_char(&mut inp).unwrap(), 'é');
    }

    #[test]
    fn decodes_three_byte_sequence() {
        // U+4E2D (中) -> E4 B8 AD
        let mut inp: Input<'_> = Partial::new(&[0xE4, 0xB8, 0xAD]);
        assert_eq!(utf8_char(&mut inp).unwrap(), '中');
    }

    #[test]
    fn decodes_four_byte_sequence() {
        // U+1F600 (😀) -> F0 9F 98 80
        let mut inp: Input<'_> = Partial::new(&[0xF0, 0x9F, 0x98, 0x80]);
        assert_eq!(utf8_char(&mut inp).unwrap(), '😀');
    }

    #[test]
    fn incomplete_on_truncated_multibyte() {
        // First byte of a 3-byte sequence, nothing else.
        let mut inp: Input<'_> = Partial::new(&[0xE4]);
        let err = utf8_char(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn rejects_invalid_lead_byte() {
        let mut inp: Input<'_> = Partial::new(&[0xFF]);
        let err = utf8_char(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Backtrack(_)));
    }
}
