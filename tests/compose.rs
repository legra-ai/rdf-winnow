//! Integration test: compose the primitives into small downstream
//! parsers to prove `DoD` #4 — primitives are reusable across
//! **multiple** downstream grammars without copy-paste.
//!
//! We build two tiny parsers here:
//!
//! 1. An **N-Triples subject** recognizer — accepts IRI, blank node, or a
//!    triple term — using `iri_ref`, `blank_node_label`, and `triple_term_open`
//!    / `triple_term_close`.
//! 2. A **Turtle-lite literal-with-language-tag** recognizer — accepts
//!    `"body"@lang(--dir)?` — using `string_literal_quote` and `lang_tag`.
//!
//! Both parsers live entirely in this test file: no copy-paste into
//! the production N-Triples or Turtle crates. If a future change to
//! the primitives breaks either parser, the test fails first.

use rdf_winnow::{
    Input, LangDirection, LangTag, RdfResult, blank_node_label, iri_ref, lang_tag,
    string_literal_quote, triple_term_close, triple_term_open, whitespace_or_comment,
};
use winnow::Partial;
use winnow::error::ErrMode;

// ---- Parser 1: N-Triples subject -------------------------------------

#[derive(Debug, PartialEq)]
enum Subject<'a> {
    Iri(&'a [u8]),
    BlankNode(&'a [u8]),
    /// A triple term — we only recognize the delimiters and return a
    /// slice of the inner bytes.
    TripleTerm(&'a [u8]),
}

fn subject<'a>(input: &mut Input<'a>) -> RdfResult<Subject<'a>> {
    whitespace_or_comment(input)?;
    let snapshot: &[u8] = &*input;
    if snapshot.starts_with(b"<<") {
        // Triple term: <<( inner )>>.
        triple_term_open(input)?;
        // Capture the inner bytes up to but not including the close.
        let before = *input;
        consume_until_triple_term_close(input)?;
        let consumed = before.len() - input.len();
        let body = &before[..consumed - ")>>".len()];
        return Ok(Subject::TripleTerm(body));
    }
    if snapshot.starts_with(b"_:") {
        let label = blank_node_label(input)?;
        return Ok(Subject::BlankNode(label));
    }
    // Otherwise IRI.
    let body = iri_ref(input)?;
    Ok(Subject::Iri(body))
}

fn consume_until_triple_term_close(input: &mut Input<'_>) -> RdfResult<()> {
    use winnow::Parser;
    use winnow::token::any;
    loop {
        let snapshot: &[u8] = &*input;
        if snapshot.starts_with(b")>>") {
            triple_term_close(input)?;
            return Ok(());
        }
        if snapshot.is_empty() {
            return Err(ErrMode::Incomplete(winnow::error::Needed::new(1)));
        }
        let _: u8 = any.parse_next(input)?;
    }
}

// ---- Parser 2: "literal"@tag with optional direction --------------

#[derive(Debug, PartialEq)]
struct LangLiteral<'a> {
    body: &'a [u8],
    lang: LangTag<'a>,
}

fn lang_literal<'a>(input: &mut Input<'a>) -> RdfResult<LangLiteral<'a>> {
    let body = string_literal_quote(input)?;
    let lang = lang_tag(input)?;
    Ok(LangLiteral { body, lang })
}

// ---- Test cases --------------------------------------------------------

#[test]
fn subject_accepts_iri() {
    let mut inp: Input<'_> = Partial::new(b"<http://ex/> ");
    assert_eq!(subject(&mut inp).unwrap(), Subject::Iri(b"http://ex/"));
}

#[test]
fn subject_accepts_blank_node() {
    let mut inp: Input<'_> = Partial::new(b"_:foo ");
    assert_eq!(subject(&mut inp).unwrap(), Subject::BlankNode(b"foo"));
}

#[test]
fn subject_skips_leading_whitespace_and_comment() {
    let mut inp: Input<'_> = Partial::new(b"# a comment\n  <http://ex/> ");
    assert_eq!(subject(&mut inp).unwrap(), Subject::Iri(b"http://ex/"));
}

#[test]
fn subject_accepts_triple_term() {
    let mut inp: Input<'_> = Partial::new(b"<<( <s> <p> <o> )>> rest");
    let got = subject(&mut inp).unwrap();
    match got {
        Subject::TripleTerm(body) => {
            assert_eq!(body, b" <s> <p> <o> ");
        }
        other => panic!("unexpected {other:?}"),
    }
    assert_eq!(&*inp, b" rest");
}

#[test]
fn lang_literal_plain() {
    let mut inp: Input<'_> = Partial::new(b"\"hello\"@en rest");
    let got = lang_literal(&mut inp).unwrap();
    assert_eq!(got.body, b"hello");
    assert_eq!(got.lang.tag, b"en");
    assert_eq!(got.lang.direction, None);
    assert_eq!(&*inp, b" rest");
}

#[test]
fn lang_literal_with_rtl_direction() {
    let mut inp: Input<'_> = Partial::new(b"\"\xd7\xa9\xd7\x9c\xd7\x95\xd7\x9d\"@he--rtl rest");
    let got = lang_literal(&mut inp).unwrap();
    assert_eq!(got.body, b"\xd7\xa9\xd7\x9c\xd7\x95\xd7\x9d");
    assert_eq!(got.lang.tag, b"he");
    assert_eq!(got.lang.direction, Some(LangDirection::Rtl));
}

#[test]
fn lang_literal_with_echar_in_body() {
    let mut inp: Input<'_> = Partial::new(b"\"a\\nb\"@en rest");
    let got = lang_literal(&mut inp).unwrap();
    assert_eq!(got.body, br"a\nb");
    assert_eq!(got.lang.tag, b"en");
}

#[test]
fn subject_is_incomplete_on_partial_iri() {
    let mut inp: Input<'_> = Partial::new(b"<http://ex");
    let err = subject(&mut inp).unwrap_err();
    assert!(matches!(err, ErrMode::Incomplete(_)));
}

#[test]
fn lang_literal_is_incomplete_without_tag() {
    let mut inp: Input<'_> = Partial::new(b"\"hello\"");
    let err = lang_literal(&mut inp).unwrap_err();
    assert!(matches!(err, ErrMode::Incomplete(_)));
}
