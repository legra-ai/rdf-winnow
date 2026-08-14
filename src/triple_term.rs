//! RDF 1.2 triple-term delimiter primitives.
//!
//! Grammar:
//!
//! ```text
//! TripleTerm      ::= '<<(' ... ')>>'
//! ```
//!
//! These primitives match only the delimiters — the inner `s p o`
//! sequence is the caller's concern.

use winnow::Parser;
use winnow::error::StrContext;

use crate::{Input, RdfResult};

/// Match the opening delimiter `<<(`.
pub fn triple_term_open(input: &mut Input<'_>) -> RdfResult<()> {
    "<<("
        .void()
        .context(StrContext::Label("triple_term_open"))
        .parse_next(input)
}

/// Match the closing delimiter `)>>`.
pub fn triple_term_close(input: &mut Input<'_>) -> RdfResult<()> {
    ")>>"
        .void()
        .context(StrContext::Label("triple_term_close"))
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
    fn open_matches() {
        let mut inp = input(b"<<(rest");
        triple_term_open(&mut inp).unwrap();
        assert_eq!(&*inp, b"rest");
    }

    #[test]
    fn close_matches() {
        let mut inp = input(b")>>rest");
        triple_term_close(&mut inp).unwrap();
        assert_eq!(&*inp, b"rest");
    }

    #[test]
    fn open_rejects_wrong_prefix() {
        let mut inp = input(b"<<x");
        assert!(triple_term_open(&mut inp).is_err());
    }

    #[test]
    fn open_incomplete_on_partial() {
        let mut inp = input(b"<<");
        let err = triple_term_open(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }

    #[test]
    fn close_incomplete_on_partial() {
        let mut inp = input(b")>");
        let err = triple_term_close(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }
}
