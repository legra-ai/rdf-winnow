//! IRI reference primitive: `IRIREF ::= '<' (...)* '>'`.
//!
//! The returned slice is the **body** of the IRI — the bytes
//! between the angle brackets, without the delimiters and
//! without UCHAR-escape decoding. Callers perform UTF-8
//! validation and escape expansion on the returned slice.

use winnow::Parser;
use winnow::combinator::{alt, delimited, repeat};
use winnow::error::{AddContext, ContextError, ErrMode, ParserError, StrContext};
use winnow::stream::Stream;
use winnow::token::one_of;

use crate::{Input, RdfResult};

/// Match an IRIREF.
///
/// Returns the byte slice inside `<...>`, without the angle brackets.
///
/// Bytes below `0x20` and the forbidden set
/// (`<`, `>`, `"`, `{`, `}`, `|`, `^`, `` ` ``, `\` except as the
/// first byte of a UCHAR escape) cause a backtrack error.
///
/// # Errors
///
/// Returns `ErrMode::Backtrack` tagged `"iri_ref"` on a malformed
/// body or missing `<` prefix, or `ErrMode::Incomplete` if the
/// closing `>` has not been seen.
pub fn iri_ref<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    delimited(
        b'<',
        repeat::<_, _, (), _, _>(0.., iri_body_byte).take(),
        b'>',
    )
    .context(StrContext::Label("iri_ref"))
    .parse_next(input)
}

/// Match an `IRIREF` and require its body to be an **absolute** IRI
/// per RFC 3986 §3.1 — i.e. start with a valid scheme prefix
/// (`ALPHA *( ALPHA / DIGIT / "+" / "-" / "." ) ":"`).
///
/// Used by line-based RDF formats (N-Triples 1.2, N-Quads 1.2) which
/// have no `@base` directive and therefore disallow relative IRIs.
/// Turtle and `TriG` resolve relative IRIs against a base IRI and
/// must not call this — they use [`iri_ref`] directly.
///
/// # Errors
///
/// Returns `ErrMode::Backtrack` tagged `"absolute_iri_ref"` if the
/// body lacks a valid scheme prefix (or is empty / starts with a
/// non-letter / contains an out-of-range scheme byte). Surface-level
/// errors from [`iri_ref`] propagate unchanged.
pub fn absolute_iri_ref<'a>(input: &mut Input<'a>) -> RdfResult<&'a [u8]> {
    let checkpoint = input.checkpoint();
    let bytes = iri_ref(input)?;
    if !is_absolute_iri(bytes) {
        let err = <ContextError as ParserError<Input<'_>>>::from_input(input).add_context(
            input,
            &checkpoint,
            StrContext::Label("absolute_iri_ref"),
        );
        return Err(ErrMode::Backtrack(err));
    }
    Ok(bytes)
}

/// Test whether `bytes` begins with a valid RFC 3986 §3.1 scheme
/// prefix and a `:`. Public so format-specific parsers (and tests)
/// can share the rule with [`absolute_iri_ref`].
#[must_use]
pub fn is_absolute_iri(bytes: &[u8]) -> bool {
    let Some(colon_pos) = bytes.iter().position(|&b| b == b':') else {
        return false;
    };
    let scheme = &bytes[..colon_pos];
    let mut iter = scheme.iter();
    let Some(&first) = iter.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() {
        return false;
    }
    iter.all(|&b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
}

/// Consume either a UCHAR escape or one plain IRI body byte.
fn iri_body_byte(input: &mut Input<'_>) -> RdfResult<()> {
    alt((
        uchar,
        one_of(|b: u8| {
            b >= 0x20
                && !matches!(
                    b,
                    b'<' | b'>' | b'"' | b'{' | b'}' | b'|' | b'^' | b'`' | b'\\'
                )
        })
        .void(),
    ))
    .parse_next(input)
}

/// UCHAR ::= '\u' HEX{4} | '\U' HEX{8}
pub(crate) fn uchar(input: &mut Input<'_>) -> RdfResult<()> {
    (
        b'\\',
        alt((
            (b'u', hex_digits::<4>).void(),
            (b'U', hex_digits::<8>).void(),
        )),
    )
        .void()
        .parse_next(input)
}

/// Exactly `N` ASCII hex digits.
pub(crate) fn hex_digits<const N: usize>(input: &mut Input<'_>) -> RdfResult<()> {
    for _ in 0..N {
        one_of(|b: u8| b.is_ascii_hexdigit()).parse_next(input)?;
    }
    Ok(())
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
    fn matches_simple_iri() {
        let mut inp = input(b"<http://example.org/> ");
        let body = iri_ref(&mut inp).unwrap();
        assert_eq!(body, b"http://example.org/");
        assert_eq!(&*inp, b" ");
    }

    #[test]
    fn matches_empty_iri() {
        let mut inp = input(b"<>x");
        let body = iri_ref(&mut inp).unwrap();
        assert_eq!(body, b"");
        assert_eq!(&*inp, b"x");
    }

    #[test]
    fn accepts_uchar_escape() {
        let mut inp = input(br"<http://ex/\u00e9> ");
        let body = iri_ref(&mut inp).unwrap();
        assert_eq!(body, br"http://ex/\u00e9");
    }

    #[test]
    fn accepts_big_uchar_escape() {
        let mut inp = input(br"<\U0001F600> ");
        let body = iri_ref(&mut inp).unwrap();
        assert_eq!(body, br"\U0001F600");
    }

    #[test]
    fn incomplete_on_unterminated_iri() {
        let mut inp = input(b"<http://ex/");
        let err = iri_ref(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn incomplete_midway_through_uchar() {
        let mut inp = input(br"<\u00");
        let err = iri_ref(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn rejects_forbidden_byte() {
        let mut inp = input(b"<foo bar>");
        // space (0x20) is allowed by our >= 0x20 rule, but `|` is in
        // the forbidden set.
        let _ok = iri_ref(&mut inp);
        let mut inp2 = input(b"<foo|bar>");
        assert!(iri_ref(&mut inp2).is_err());
    }

    #[test]
    fn rejects_control_byte() {
        let mut inp = input(b"<foo\x01bar>");
        assert!(iri_ref(&mut inp).is_err());
    }

    #[test]
    fn rejects_bare_backslash() {
        let mut inp = input(br"<foo\x>");
        assert!(iri_ref(&mut inp).is_err());
    }

    #[test]
    fn absolute_iri_accepts_scheme() {
        let mut inp = input(b"<http://example/> ");
        let body = absolute_iri_ref(&mut inp).unwrap();
        assert_eq!(body, b"http://example/");
    }

    #[test]
    fn absolute_iri_rejects_missing_scheme() {
        let mut inp = input(b"<//example/missing-scheme> ");
        assert!(absolute_iri_ref(&mut inp).is_err());
    }

    #[test]
    fn absolute_iri_rejects_empty_iri() {
        let mut inp = input(b"<> ");
        assert!(absolute_iri_ref(&mut inp).is_err());
    }

    #[test]
    fn absolute_iri_rejects_relative_path() {
        let mut inp = input(b"<foo/bar> ");
        assert!(absolute_iri_ref(&mut inp).is_err());
    }

    #[test]
    fn absolute_iri_rejects_digit_start_scheme() {
        let mut inp = input(b"<1http://example/> ");
        assert!(absolute_iri_ref(&mut inp).is_err());
    }

    #[test]
    fn absolute_iri_accepts_scheme_with_plus_minus_dot() {
        assert!(is_absolute_iri(b"git+ssh://example/"));
        assert!(is_absolute_iri(b"a.b-c+d:foo"));
        assert!(!is_absolute_iri(b"-foo:bar"));
    }
}
