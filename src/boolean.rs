//! Boolean literal primitive.
//!
//! Grammar: `BooleanLiteral ::= 'true' | 'false'`.

use winnow::Parser;
use winnow::combinator::alt;
use winnow::error::StrContext;

use crate::{Input, RdfResult};

/// Match a Turtle boolean keyword literal.
pub fn boolean_literal(input: &mut Input<'_>) -> RdfResult<bool> {
    alt(("true".map(|_: &[u8]| true), "false".map(|_: &[u8]| false)))
        .context(StrContext::Label("boolean_literal"))
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
    fn matches_true() {
        let mut inp = input(b"true ");
        assert!(boolean_literal(&mut inp).unwrap());
        assert_eq!(&*inp, b" ");
    }

    #[test]
    fn matches_false() {
        let mut inp = input(b"false ");
        assert!(!boolean_literal(&mut inp).unwrap());
        assert_eq!(&*inp, b" ");
    }

    #[test]
    fn rejects_other_keywords() {
        let mut inp = input(b"True ");
        assert!(boolean_literal(&mut inp).is_err());
    }

    #[test]
    fn incomplete_on_partial_match() {
        let mut inp = input(b"tru");
        let err = boolean_literal(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }
}
