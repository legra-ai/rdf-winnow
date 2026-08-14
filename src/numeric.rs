//! Numeric-literal primitive.
//!
//! Grammar (Turtle 1.2):
//!
//! ```text
//! INTEGER  ::= [+-]? [0-9]+
//! DECIMAL  ::= [+-]? [0-9]* '.' [0-9]+
//! DOUBLE   ::= [+-]? ( [0-9]+ '.' [0-9]* EXPONENT
//!                    | '.' [0-9]+ EXPONENT
//!                    | [0-9]+ EXPONENT )
//! EXPONENT ::= [eE] [+-]? [0-9]+
//! ```
//!
//! [`numeric_literal`] returns a [`NumericLiteral`] carrying the raw
//! byte slice plus a [`NumericKind`] discriminant. Callers decode the
//! lexical form themselves.

use winnow::Parser;
use winnow::combinator::{alt, opt, preceded, repeat};
use winnow::error::StrContext;
use winnow::stream::{Offset, Stream};
use winnow::token::{one_of, take};

use crate::{Input, RdfResult};

/// Discriminates between Turtle's three numeric literal categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumericKind {
    /// No fractional part, no exponent.
    Integer,
    /// Fractional part, no exponent.
    Decimal,
    /// Has an exponent.
    Double,
}

/// A parsed numeric literal: the raw lexical slice plus its kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericLiteral<'a> {
    /// Raw lexical form, including any leading `+`/`-` sign.
    pub raw: &'a [u8],
    /// Which kind of numeric literal this is.
    pub kind: NumericKind,
}

/// Match a numeric literal.
pub fn numeric_literal<'a>(input: &mut Input<'a>) -> RdfResult<NumericLiteral<'a>> {
    let start = input.checkpoint();

    // Optional sign.
    let _ = opt(one_of(b"+-")).parse_next(input)?;

    // Three mutually exclusive shapes:
    //   A: [0-9]+ ('.' [0-9]*)? (EXPONENT)?
    //   B: '.' [0-9]+ (EXPONENT)?
    let kind = alt((shape_leading_digits, shape_leading_dot))
        .context(StrContext::Label("numeric_literal"))
        .parse_next(input)?;

    let length = input.offset_from(&start);
    input.reset(&start);
    let raw: &[u8] = take(length).parse_next(input)?;
    Ok(NumericLiteral { raw, kind })
}

/// `[0-9]+ ('.' [0-9]*)? EXPONENT?`.
fn shape_leading_digits(input: &mut Input<'_>) -> RdfResult<NumericKind> {
    // [0-9]+
    let _ = one_of(|b: u8| b.is_ascii_digit()).parse_next(input)?;
    let _: () = repeat(0.., one_of(|b: u8| b.is_ascii_digit())).parse_next(input)?;

    // Optional '.' [0-9]* (decimal part).
    let has_dot = opt((
        b'.',
        repeat::<_, _, (), _, _>(0.., one_of(|b: u8| b.is_ascii_digit())),
    ))
    .parse_next(input)?
    .is_some();

    // Optional exponent.
    let has_exp = opt(exponent).parse_next(input)?.is_some();

    Ok(match (has_dot, has_exp) {
        (_, true) => NumericKind::Double,
        (true, false) => NumericKind::Decimal,
        (false, false) => NumericKind::Integer,
    })
}

/// `'.' [0-9]+ EXPONENT?`.
fn shape_leading_dot(input: &mut Input<'_>) -> RdfResult<NumericKind> {
    b'.'.parse_next(input)?;
    let _ = one_of(|b: u8| b.is_ascii_digit()).parse_next(input)?;
    let _: () = repeat(0.., one_of(|b: u8| b.is_ascii_digit())).parse_next(input)?;
    let has_exp = opt(exponent).parse_next(input)?.is_some();
    Ok(if has_exp {
        NumericKind::Double
    } else {
        NumericKind::Decimal
    })
}

/// `EXPONENT ::= [eE] [+-]? [0-9]+`.
fn exponent(input: &mut Input<'_>) -> RdfResult<()> {
    (
        one_of(b"eE"),
        opt(one_of(b"+-")),
        preceded(
            one_of(|b: u8| b.is_ascii_digit()),
            repeat::<_, _, (), _, _>(0.., one_of(|b: u8| b.is_ascii_digit())),
        ),
    )
        .void()
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
    fn integer_positive() {
        let mut inp = input(b"42 ");
        let n = numeric_literal(&mut inp).unwrap();
        assert_eq!(n.raw, b"42");
        assert_eq!(n.kind, NumericKind::Integer);
    }

    #[test]
    fn integer_with_sign() {
        let mut inp = input(b"-7 ");
        let n = numeric_literal(&mut inp).unwrap();
        assert_eq!(n.raw, b"-7");
        assert_eq!(n.kind, NumericKind::Integer);
    }

    #[test]
    fn decimal_with_fraction() {
        let mut inp = input(b"3.14 ");
        let n = numeric_literal(&mut inp).unwrap();
        assert_eq!(n.raw, b"3.14");
        assert_eq!(n.kind, NumericKind::Decimal);
    }

    #[test]
    fn decimal_leading_dot() {
        let mut inp = input(b".5 ");
        let n = numeric_literal(&mut inp).unwrap();
        assert_eq!(n.raw, b".5");
        assert_eq!(n.kind, NumericKind::Decimal);
    }

    #[test]
    fn double_exponent_form() {
        let mut inp = input(b"1e10 ");
        let n = numeric_literal(&mut inp).unwrap();
        assert_eq!(n.raw, b"1e10");
        assert_eq!(n.kind, NumericKind::Double);
    }

    #[test]
    fn double_mixed_form() {
        let mut inp = input(b"+1.5E-3 ");
        let n = numeric_literal(&mut inp).unwrap();
        assert_eq!(n.raw, b"+1.5E-3");
        assert_eq!(n.kind, NumericKind::Double);
    }

    #[test]
    fn rejects_sign_only() {
        let mut inp = input(b"-x ");
        assert!(numeric_literal(&mut inp).is_err());
    }

    #[test]
    fn incomplete_midway() {
        let mut inp = input(b"3.1");
        let err = numeric_literal(&mut inp).unwrap_err();
        assert!(matches!(err, ErrMode::Incomplete(_)));
    }
}
