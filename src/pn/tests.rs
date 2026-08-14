use winnow::Partial;
use winnow::error::ErrMode;

use super::*;
use crate::Input;

fn input(bytes: &[u8]) -> Input<'_> {
    Partial::new(bytes)
}

#[test]
fn pn_chars_base_matches_ascii_letters() {
    let mut inp = input(b"A");
    assert_eq!(pn_chars_base(&mut inp).unwrap(), 'A');
    let mut inp = input(b"z");
    assert_eq!(pn_chars_base(&mut inp).unwrap(), 'z');
}

#[test]
fn pn_chars_base_rejects_digit_and_underscore() {
    let mut inp = input(b"0");
    assert!(pn_chars_base(&mut inp).is_err());
    let mut inp = input(b"_");
    assert!(pn_chars_base(&mut inp).is_err());
}

#[test]
fn pn_chars_u_accepts_underscore() {
    let mut inp = input(b"_");
    assert_eq!(pn_chars_u(&mut inp).unwrap(), '_');
}

#[test]
fn pn_chars_accepts_digit_and_hyphen() {
    let mut inp = input(b"5");
    assert_eq!(pn_chars(&mut inp).unwrap(), '5');
    let mut inp = input(b"-");
    assert_eq!(pn_chars(&mut inp).unwrap(), '-');
}

#[test]
fn pn_chars_base_accepts_non_ascii_letter() {
    // U+00E9 é
    let mut inp = input(&[0xC3, 0xA9]);
    assert_eq!(pn_chars_base(&mut inp).unwrap(), 'é');
}

#[test]
fn pn_prefix_matches_ascii_ident() {
    let mut inp = input(b"foo:");
    let s = pn_prefix(&mut inp).unwrap();
    assert_eq!(s, b"foo");
    assert_eq!(&*inp, b":");
}

#[test]
fn pn_prefix_allows_internal_dots_but_strips_trailing() {
    let mut inp = input(b"foo.bar.:");
    let s = pn_prefix(&mut inp).unwrap();
    assert_eq!(s, b"foo.bar");
    assert_eq!(&*inp, b".:");
}

#[test]
fn pn_prefix_rejects_leading_digit() {
    let mut inp = input(b"1foo:");
    assert!(pn_prefix(&mut inp).is_err());
}

#[test]
fn pn_local_starts_with_digit_or_colon() {
    let mut inp = input(b"123 ");
    let s = pn_local(&mut inp).unwrap();
    assert_eq!(s, b"123");

    let mut inp = input(b":foo ");
    let s = pn_local(&mut inp).unwrap();
    assert_eq!(s, b":foo");
}

#[test]
fn pn_local_allows_percent_escape() {
    let mut inp = input(b"foo%20bar ");
    let s = pn_local(&mut inp).unwrap();
    assert_eq!(s, b"foo%20bar");
}

#[test]
fn pn_local_allows_pn_local_esc() {
    let mut inp = input(br"foo\~bar ");
    let s = pn_local(&mut inp).unwrap();
    assert_eq!(s, br"foo\~bar");
}

#[test]
fn pn_local_strips_trailing_dot() {
    let mut inp = input(b"foo.bar. ");
    let s = pn_local(&mut inp).unwrap();
    assert_eq!(s, b"foo.bar");
    assert_eq!(&*inp, b". ");
}

#[test]
fn pn_prefix_incomplete_at_eof() {
    let mut inp = input(b"foo");
    let err = pn_prefix(&mut inp).unwrap_err();
    assert!(matches!(err, ErrMode::Incomplete(_)));
}
