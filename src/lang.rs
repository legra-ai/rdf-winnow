//! Language-tag primitive.
//!
//! Grammar (RDF 1.2 / BCP47, simplified):
//!
//! ```text
//! LANGTAG ::= '@' [a-zA-Z]{1,8} ('-' [a-zA-Z0-9]{1,8})* ('--' [a-zA-Z]+)?
//! ```
//!
//! Each subtag — the primary language subtag and each `-`-separated
//! continuation — is at most 8 characters long, mirroring the
//! BCP47 / RFC 5646 §2.2 maximum. The optional `--` suffix encodes an
//! RDF 1.2 base-direction hint (`--ltr` or `--rtl`). Any other
//! keyword in the suffix position is rejected.

use winnow::Parser;
use winnow::combinator::{alt, cut_err, opt, preceded, repeat};
use winnow::error::{AddContext, ContextError, ErrMode, ParserError, StrContext};
use winnow::stream::{Offset, Stream};
use winnow::token::{one_of, take};

use crate::{Input, RdfResult};

/// BCP47 / RFC 5646 §2.2 cap on a single subtag's length.
const SUBTAG_MAX_LEN: usize = 8;

/// Direction hint attached to an RDF 1.2 directional language-tagged
/// literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LangDirection {
    /// Left-to-right (`--ltr`).
    Ltr,
    /// Right-to-left (`--rtl`).
    Rtl,
}

/// A parsed language tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LangTag<'a> {
    /// The language tag bytes (without the leading `@`, without the
    /// optional `--ltr`/`--rtl` suffix).
    pub tag: &'a [u8],
    /// Optional RDF 1.2 base-direction hint.
    pub direction: Option<LangDirection>,
}

/// Match a language tag starting with `@`.
pub fn lang_tag<'a>(input: &mut Input<'a>) -> RdfResult<LangTag<'a>> {
    b'@'.parse_next(input)?;

    let tag_start = input.checkpoint();

    // Primary language subtag: 1..=SUBTAG_MAX_LEN ASCII letters.
    let _: () =
        repeat(1..=SUBTAG_MAX_LEN, one_of(|b: u8| b.is_ascii_alphabetic())).parse_next(input)?;

    // ('-' [a-zA-Z0-9]{1,8})*
    let _: () = repeat(0.., subtag).parse_next(input)?;

    let tag_len = input.offset_from(&tag_start);
    input.reset(&tag_start);
    let tag: &[u8] = take(tag_len).parse_next(input)?;

    // Reject if the next byte continues an alphabetic run — that
    // means the primary subtag was longer than SUBTAG_MAX_LEN.
    if input
        .first()
        .is_some_and(|b: &u8| b.is_ascii_alphanumeric())
    {
        let err = <ContextError as ParserError<Input<'_>>>::from_input(input).add_context(
            input,
            &tag_start,
            StrContext::Label("lang_tag_subtag_len"),
        );
        return Err(ErrMode::Backtrack(err));
    }

    // Optional direction suffix '--ltr' | '--rtl'.
    let direction = opt(direction_suffix)
        .context(StrContext::Label("lang_tag_direction"))
        .parse_next(input)?;

    Ok(LangTag { tag, direction })
}

fn subtag(input: &mut Input<'_>) -> RdfResult<()> {
    (
        b'-',
        repeat::<_, _, (), _, _>(
            1..=SUBTAG_MAX_LEN,
            one_of(|b: u8| b.is_ascii_alphanumeric()),
        ),
    )
        .void()
        .parse_next(input)
}

fn direction_suffix(input: &mut Input<'_>) -> RdfResult<LangDirection> {
    // Once `--` is seen, the suffix is committed — any keyword other
    // than `ltr` / `rtl` is a hard error, not a missed optional.
    preceded(
        b"--",
        cut_err(alt((
            "ltr".map(|_: &[u8]| LangDirection::Ltr),
            "rtl".map(|_: &[u8]| LangDirection::Rtl),
        ))),
    )
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
    fn matches_plain_tag() {
        let mut inp = input(b"@en ");
        let t = lang_tag(&mut inp).unwrap();
        assert_eq!(t.tag, b"en");
        assert_eq!(t.direction, None);
        assert_eq!(&*inp, b" ");
    }

    #[test]
    fn matches_tag_with_subtag() {
        let mut inp = input(b"@en-US ");
        let t = lang_tag(&mut inp).unwrap();
        assert_eq!(t.tag, b"en-US");
        assert_eq!(t.direction, None);
    }

    #[test]
    fn matches_tag_with_direction_ltr() {
        let mut inp = input(b"@ar--ltr ");
        let t = lang_tag(&mut inp).unwrap();
        assert_eq!(t.tag, b"ar");
        assert_eq!(t.direction, Some(LangDirection::Ltr));
    }

    #[test]
    fn matches_tag_with_direction_rtl() {
        let mut inp = input(b"@he-IL--rtl ");
        let t = lang_tag(&mut inp).unwrap();
        assert_eq!(t.tag, b"he-IL");
        assert_eq!(t.direction, Some(LangDirection::Rtl));
    }

    #[test]
    fn rejects_unknown_direction() {
        let mut inp = input(b"@en--xxx ");
        // --xxx is not a valid direction; parse should fail.
        assert!(lang_tag(&mut inp).is_err());
    }

    #[test]
    fn rejects_missing_at_sign() {
        let mut inp = input(b"en ");
        assert!(lang_tag(&mut inp).is_err());
    }

    #[test]
    fn incomplete_on_partial_tag() {
        let mut inp = input(b"@en-U");
        let err = lang_tag(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn rejects_subtag_longer_than_eight_chars() {
        let mut inp = input(b"@cantbethislong ");
        assert!(lang_tag(&mut inp).is_err());
    }

    #[test]
    fn accepts_eight_char_subtag() {
        let mut inp = input(b"@abcdefgh ");
        let t = lang_tag(&mut inp).unwrap();
        assert_eq!(t.tag, b"abcdefgh");
    }

    #[test]
    fn rejects_extension_subtag_longer_than_eight() {
        let mut inp = input(b"@en-cantbethislong ");
        assert!(lang_tag(&mut inp).is_err());
    }
}
