//! Blank-node label primitive.
//!
//! Grammar: `BLANK_NODE_LABEL ::= '_:' (PN_CHARS_U | [0-9]) ((PN_CHARS | '.')*
//! PN_CHARS)?`. Returns the byte slice of the label **without** the `_:`
//! prefix.

use winnow::Parser;
use winnow::error::{AddContext, ContextError, ErrMode, ParserError, StrContext};
use winnow::stream::{Offset, Stream};
use winnow::token::take;

use crate::pn::{is_pn_chars, is_pn_chars_u};
use crate::utf8::utf8_char;
use crate::{Input, RdfResult};

/// Match a blank-node label, returning the label bytes (without the
/// leading `_:`).
pub fn blank_node_label<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    // '_:' prefix.
    (b'_', b':').parse_next(input)?;

    let label_start = input.checkpoint();

    // Head: PN_CHARS_U | [0-9].
    {
        let before_head = input.checkpoint();
        match utf8_char(input) {
            Ok(c) if is_pn_chars_u(c) || c.is_ascii_digit() => {}
            Ok(_) => {
                input.reset(&before_head);
                return Err(label_err(input));
            }
            Err(ErrMode::Incomplete(needed)) => {
                input.reset(&before_head);
                return Err(ErrMode::Incomplete(needed));
            }
            Err(e) => {
                input.reset(&before_head);
                return Err(e);
            }
        }
    }

    // Tail: (PN_CHARS | '.')* PN_CHARS — strip trailing dots.
    let mut last_ok = input.checkpoint();
    loop {
        let before = input.checkpoint();
        match utf8_char(input) {
            Ok('.') => {
                // Tentative — keep going; may be followed by PN_CHARS.
            }
            Ok(c) if is_pn_chars(c) => {
                last_ok = input.checkpoint();
            }
            Err(ErrMode::Incomplete(needed)) => {
                input.reset(&before);
                return Err(ErrMode::Incomplete(needed));
            }
            Ok(_) | Err(_) => {
                input.reset(&before);
                break;
            }
        }
    }
    input.reset(&last_ok);

    let length = input.offset_from(&label_start);
    input.reset(&label_start);
    let slice: &[u8] = take::<_, _, ErrMode<ContextError>>(length)
        .parse_next(input)
        .expect("offset within known prefix must be takeable");
    Ok(slice)
}

fn label_err(input: &Input<'_>) -> ErrMode<ContextError> {
    let checkpoint = input.checkpoint();
    let err = <ContextError as ParserError<Input<'_>>>::from_input(input);
    ErrMode::Backtrack(err.add_context(input, &checkpoint, StrContext::Label("blank_node_label")))
}

#[cfg(test)]
mod tests {
    use winnow::Partial;
    use winnow::error::ErrMode;

    use super::*;

    fn input(bytes: &[u8]) -> Input<'_> {
        Partial::new(bytes)
    }

    #[test]
    fn matches_simple_label() {
        let mut inp = input(b"_:foo ");
        let s = blank_node_label(&mut inp).unwrap();
        assert_eq!(s, b"foo");
        assert_eq!(&*inp, b" ");
    }

    #[test]
    fn matches_label_starting_with_digit() {
        let mut inp = input(b"_:1abc ");
        let s = blank_node_label(&mut inp).unwrap();
        assert_eq!(s, b"1abc");
    }

    #[test]
    fn matches_label_with_hyphen_and_underscore() {
        let mut inp = input(b"_:foo-bar_baz ");
        let s = blank_node_label(&mut inp).unwrap();
        assert_eq!(s, b"foo-bar_baz");
    }

    #[test]
    fn strips_trailing_dot() {
        let mut inp = input(b"_:foo.bar. ");
        let s = blank_node_label(&mut inp).unwrap();
        assert_eq!(s, b"foo.bar");
        assert_eq!(&*inp, b". ");
    }

    #[test]
    fn rejects_empty_label() {
        let mut inp = input(b"_: ");
        assert!(blank_node_label(&mut inp).is_err());
    }

    #[test]
    fn rejects_missing_prefix() {
        let mut inp = input(b"foo ");
        assert!(blank_node_label(&mut inp).is_err());
    }

    #[test]
    fn incomplete_after_prefix_only() {
        let mut inp = input(b"_:");
        let err = blank_node_label(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn incomplete_at_eof_after_run() {
        let mut inp = input(b"_:foo");
        let err = blank_node_label(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }
}
