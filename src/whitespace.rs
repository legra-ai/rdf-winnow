//! Whitespace and comment primitives shared across RDF text parsers.
//!
//! Turtle and `TriG` allow whitespace and `#`-comments between every
//! grammar production; N-Triples and N-Quads allow them at line level.
//! The two forms here — [`whitespace`] (one-or-more whitespace bytes)
//! and [`whitespace_or_comment`] (zero-or-more whitespace or comment
//! runs) — cover both.

use winnow::Parser;
use winnow::combinator::{alt, repeat};
use winnow::error::StrContext;
use winnow::token::{one_of, take_till};

use crate::{Input, RdfResult};

/// Consume one or more ASCII whitespace bytes (space, tab, CR, LF).
///
/// # Errors
///
/// Returns `ErrMode::Backtrack` if the first byte is not whitespace,
/// or `ErrMode::Incomplete` on partial input whose tail is all
/// whitespace (more whitespace might follow on the next chunk).
pub fn whitespace(input: &mut Input<'_>) -> RdfResult<()> {
    repeat::<_, _, (), _, _>(1.., one_of(b" \t\n\r"))
        .context(StrContext::Label("whitespace"))
        .parse_next(input)
}

/// Consume a `#`-prefixed line comment up to (but not including) the
/// terminating newline.
///
/// # Errors
///
/// Returns `ErrMode::Backtrack` if the first byte is not `#`, or
/// `ErrMode::Incomplete` if no newline is seen before the buffer ends.
pub fn comment(input: &mut Input<'_>) -> RdfResult<()> {
    (b'#', take_till(0.., b"\n\r"))
        .void()
        .context(StrContext::Label("comment"))
        .parse_next(input)
}

/// Consume zero or more runs of whitespace and comments, in any order.
///
/// # Errors
///
/// Never returns `Backtrack` (zero occurrences is valid). Returns
/// `ErrMode::Incomplete` if the buffer tail is ambiguous (whitespace
/// or an unterminated comment that might continue).
pub fn whitespace_or_comment(input: &mut Input<'_>) -> RdfResult<()> {
    repeat::<_, _, (), _, _>(0.., alt((one_of(b" \t\n\r").void(), comment))).parse_next(input)
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
    fn whitespace_matches_spaces_and_tabs() {
        let mut inp = input(b"  \t \nfoo");
        whitespace(&mut inp).unwrap();
        assert_eq!(&*inp, b"foo");
    }

    #[test]
    fn whitespace_requires_at_least_one() {
        let mut inp = input(b"foo");
        assert!(whitespace(&mut inp).is_err());
    }

    #[test]
    fn whitespace_incomplete_on_partial_run() {
        let mut inp = input(b"   ");
        let err = whitespace(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn comment_consumes_to_newline() {
        let mut inp = input(b"# hello world\nfoo");
        comment(&mut inp).unwrap();
        assert_eq!(&*inp, b"\nfoo");
    }

    #[test]
    fn comment_incomplete_on_unterminated_line() {
        let mut inp = input(b"# partial");
        let err = comment(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn whitespace_or_comment_handles_mixed_runs() {
        let mut inp = input(b" \t# first\n  # second\n\nfoo");
        whitespace_or_comment(&mut inp).unwrap();
        assert_eq!(&*inp, b"foo");
    }

    #[test]
    fn whitespace_or_comment_accepts_empty() {
        let mut inp = input(b"foo");
        whitespace_or_comment(&mut inp).unwrap();
        assert_eq!(&*inp, b"foo");
    }
}
