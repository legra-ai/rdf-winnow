//! String-literal primitives.
//!
//! Implements the four Turtle 1.2 string literal forms:
//!
//! ```text
//! STRING_LITERAL_QUOTE            ::= '"'   ( [^"\\#x0A#x0D] | ECHAR | UCHAR )* '"'
//! STRING_LITERAL_SINGLE_QUOTE     ::= '\''  ( [^'\\#x0A#x0D] | ECHAR | UCHAR )* '\''
//! STRING_LITERAL_LONG_QUOTE       ::= '"""' ( ('"' | '""')? ( [^"\\] | ECHAR | UCHAR ) )* '"""'
//! STRING_LITERAL_LONG_SINGLE_QUOTE::= '\'\'\'' ( ('\'' | '\'\'')? ( [^'\\] | ECHAR | UCHAR ) )* '\'\'\''
//! ```
//!
//! Each primitive returns the **raw body** slice — the bytes between
//! the opening and closing delimiter, with `ECHAR`/`UCHAR` escapes
//! left in lexical form. Decoding is left to the caller.

use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::{AddContext, ContextError, ErrMode, Needed, ParserError, StrContext};
use winnow::stream::{Offset, Stream};
use winnow::token::{one_of, take};

use crate::iri::{hex_digits, uchar};
use crate::{Input, RdfResult};

/// `STRING_LITERAL_QUOTE ::= '"' ... '"'`.
pub fn string_literal_quote<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    short_string(input, b'"', "string_literal_quote")
}

/// `STRING_LITERAL_SINGLE_QUOTE ::= '\'' ... '\''`.
pub fn string_literal_single_quote<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    short_string(input, b'\'', "string_literal_single_quote")
}

/// `STRING_LITERAL_LONG_QUOTE ::= '"""' ... '"""'`.
pub fn string_literal_long_quote<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    long_string(input, b'"', "string_literal_long_quote")
}

/// `STRING_LITERAL_LONG_SINGLE_QUOTE ::= '\'\'\'' ... '\'\'\''`.
pub fn string_literal_long_single_quote<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    long_string(input, b'\'', "string_literal_long_single_quote")
}

fn short_string<'a>(input: &mut Input<'a>, quote: u8, label: &'static str) -> RdfResult<&'a [u8]> {
    one_of([quote])
        .void()
        .parse_next(input)
        .map_err(|e| add_label(e, input, label))?;

    let body_start = input.checkpoint();
    loop {
        let before = input.checkpoint();
        let slice: &[u8] = &*input;
        if slice.is_empty() {
            return Err(ErrMode::Incomplete(Needed::new(1)));
        }
        match slice[0] {
            b if b == quote => {
                let body_len = input.offset_from(&body_start);
                input.reset(&body_start);
                let body: &[u8] = take(body_len).parse_next(input)?;
                // Consume the closing quote.
                let _ = take::<_, _, ErrMode<ContextError>>(1usize).parse_next(input)?;
                return Ok(body);
            }
            b'\n' | b'\r' => {
                return Err(backtrack(input, label));
            }
            b'\\' => {
                // Either ECHAR or UCHAR. Attempt UCHAR; fall back to
                // ECHAR.
                match try_uchar_or_echar(input) {
                    Ok(()) => {}
                    Err(ErrMode::Incomplete(needed)) => {
                        input.reset(&before);
                        return Err(ErrMode::Incomplete(needed));
                    }
                    Err(e) => {
                        input.reset(&before);
                        return Err(e);
                    }
                }
            }
            _ => {
                let _ = take::<_, _, ErrMode<ContextError>>(1usize).parse_next(input)?;
            }
        }
    }
}

fn long_string<'a>(input: &mut Input<'a>, quote: u8, label: &'static str) -> RdfResult<&'a [u8]> {
    let opener = [quote, quote, quote];
    // Match opening delimiter.
    expect_literal(input, &opener, label)?;

    let body_start = input.checkpoint();
    loop {
        let before = input.checkpoint();
        let slice: &[u8] = &*input;
        if slice.is_empty() {
            return Err(ErrMode::Incomplete(Needed::new(1)));
        }
        match slice[0] {
            b if b == quote => {
                // Look ahead: 1, 2, or 3 quotes. Three quotes = close.
                if slice.len() < 3 {
                    return Err(ErrMode::Incomplete(Needed::new(3 - slice.len())));
                }
                if slice[1] == quote && slice[2] == quote {
                    let body_len = input.offset_from(&body_start);
                    input.reset(&body_start);
                    let body: &[u8] = take(body_len).parse_next(input)?;
                    let _ = take::<_, _, ErrMode<ContextError>>(3usize).parse_next(input)?;
                    return Ok(body);
                }
                // One or two inner quotes — consume them and continue.
                let consume: usize = if slice[1] == quote { 2 } else { 1 };
                let _ = take::<_, _, ErrMode<ContextError>>(consume).parse_next(input)?;
            }
            b'\\' => match try_uchar_or_echar(input) {
                Ok(()) => {}
                Err(ErrMode::Incomplete(needed)) => {
                    input.reset(&before);
                    return Err(ErrMode::Incomplete(needed));
                }
                Err(e) => {
                    input.reset(&before);
                    return Err(e);
                }
            },
            _ => {
                let _ = take::<_, _, ErrMode<ContextError>>(1usize).parse_next(input)?;
            }
        }
    }
}

/// Try a UCHAR first (`\uXXXX` / `\UXXXXXXXX`); fall back to ECHAR
/// (`\tbnrf"'\\`).
fn try_uchar_or_echar(input: &mut Input<'_>) -> RdfResult<()> {
    let before = input.checkpoint();
    match uchar(input) {
        Ok(()) => Ok(()),
        Err(ErrMode::Incomplete(needed)) => {
            input.reset(&before);
            Err(ErrMode::Incomplete(needed))
        }
        Err(_) => {
            input.reset(&before);
            echar(input)
        }
    }
}

/// `ECHAR ::= '\' [tbnrf"'\\]`.
fn echar(input: &mut Input<'_>) -> RdfResult<()> {
    (b'\\', one_of(is_echar_byte)).void().parse_next(input)
}

fn is_echar_byte(b: u8) -> bool {
    matches!(b, b't' | b'b' | b'n' | b'r' | b'f' | b'"' | b'\'' | b'\\')
}

/// Expect `literal` bytes or back-track with a contextual label.
fn expect_literal(input: &mut Input<'_>, literal: &[u8], label: &'static str) -> RdfResult<()> {
    let before = input.checkpoint();
    let slice: &[u8] = &*input;
    if slice.len() < literal.len() {
        // Might be a prefix match — ask for more bytes.
        if slice == &literal[..slice.len()] {
            return Err(ErrMode::Incomplete(Needed::new(
                literal.len() - slice.len(),
            )));
        }
        input.reset(&before);
        return Err(backtrack(input, label));
    }
    if &slice[..literal.len()] != literal {
        input.reset(&before);
        return Err(backtrack(input, label));
    }
    let _ = take::<_, _, ErrMode<ContextError>>(literal.len()).parse_next(input)?;
    Ok(())
}

/// Convenience used by the escape-scanner when it sees `\` but wants
/// a bounded lookahead — not currently used externally but kept as
/// an internal helper for future extensions (e.g. enforcing exactly
/// four hex digits after `\u`).
#[allow(dead_code)]
fn expect_hex_digits<const N: usize>(input: &mut Input<'_>) -> RdfResult<()> {
    hex_digits::<N>(input)
}

fn add_label(
    mut err: ErrMode<ContextError>,
    input: &Input<'_>,
    label: &'static str,
) -> ErrMode<ContextError> {
    if let ErrMode::Backtrack(ref mut e) | ErrMode::Cut(ref mut e) = err {
        let checkpoint = input.checkpoint();
        let cloned = std::mem::replace(e, ContextError::new());
        *e = cloned.add_context(input, &checkpoint, StrContext::Label(label));
    }
    err
}

fn backtrack(input: &Input<'_>, label: &'static str) -> ErrMode<ContextError> {
    let checkpoint = input.checkpoint();
    let err = <ContextError as ParserError<Input<'_>>>::from_input(input);
    ErrMode::Backtrack(err.add_context(input, &checkpoint, StrContext::Label(label)))
}

/// Match any of the four string literal forms, preferring the long
/// forms (`"""` / `'''`) over the short forms since the long prefix
/// would otherwise be interpreted as an empty short string followed
/// by a body.
pub fn any_string_literal<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    alt((
        string_literal_long_quote,
        string_literal_long_single_quote,
        string_literal_quote,
        string_literal_single_quote,
    ))
    .context(StrContext::Label("string_literal"))
    .parse_next(input)
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
    fn short_quote_plain() {
        let mut inp = input(br#""hello"!"#);
        let body = string_literal_quote(&mut inp).unwrap();
        assert_eq!(body, b"hello");
        assert_eq!(&*inp, b"!");
    }

    #[test]
    fn short_quote_empty() {
        let mut inp = input(br#""" rest"#);
        let body = string_literal_quote(&mut inp).unwrap();
        assert_eq!(body, b"");
    }

    #[test]
    fn short_quote_with_echar() {
        let mut inp = input(br#""a\nb\"c""#);
        let body = string_literal_quote(&mut inp).unwrap();
        assert_eq!(body, br#"a\nb\"c"#);
    }

    #[test]
    fn short_quote_with_uchar() {
        let mut inp = input(br#""a\u00e9b"!"#);
        let body = string_literal_quote(&mut inp).unwrap();
        assert_eq!(body, br"a\u00e9b");
    }

    #[test]
    fn short_quote_rejects_raw_newline() {
        let mut inp = input(b"\"line1\nline2\"");
        assert!(string_literal_quote(&mut inp).is_err());
    }

    #[test]
    fn short_quote_incomplete_on_unterminated() {
        let mut inp = input(br#""hello"#);
        let err = string_literal_quote(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn single_quote_plain() {
        let mut inp = input(b"'hello'!");
        let body = string_literal_single_quote(&mut inp).unwrap();
        assert_eq!(body, b"hello");
    }

    #[test]
    fn long_quote_plain() {
        let mut inp = input(br#""""foo bar""" rest"#);
        let body = string_literal_long_quote(&mut inp).unwrap();
        assert_eq!(body, b"foo bar");
        assert_eq!(&*inp, b" rest");
    }

    #[test]
    fn long_quote_allows_inner_quote() {
        let mut inp = input(br#""""a"b""c""" x"#);
        let body = string_literal_long_quote(&mut inp).unwrap();
        assert_eq!(body, br#"a"b""c"#);
    }

    #[test]
    fn long_quote_spans_newlines() {
        let mut inp = input(b"\"\"\"line1\nline2\"\"\"x");
        let body = string_literal_long_quote(&mut inp).unwrap();
        assert_eq!(body, b"line1\nline2");
    }

    #[test]
    fn long_single_quote_plain() {
        let mut inp = input(b"'''foo''' rest");
        let body = string_literal_long_single_quote(&mut inp).unwrap();
        assert_eq!(body, b"foo");
    }

    #[test]
    fn long_quote_incomplete_on_truncated_closer() {
        let mut inp = input(br#""""foo""#);
        let err = string_literal_long_quote(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn any_string_literal_picks_long_form() {
        let mut inp = input(br#""""triple""" rest"#);
        let body = any_string_literal(&mut inp).unwrap();
        assert_eq!(body, b"triple");
    }

    #[test]
    fn any_string_literal_picks_short_form() {
        let mut inp = input(br#""short" rest"#);
        let body = any_string_literal(&mut inp).unwrap();
        assert_eq!(body, b"short");
    }
}
