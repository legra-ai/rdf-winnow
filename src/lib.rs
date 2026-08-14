#![doc = include_str!("../README.md")]
// Every primitive in this crate shares a single well-defined error
// contract, documented in the crate README and at the module level:
//   * `ErrMode::Incomplete(Needed)` on partial input ending mid-production (so the feeder can
//     supply more bytes).
//   * `ErrMode::Backtrack(ContextError)` carrying a `StrContext::Label` naming the production on
//     any syntactic mismatch.
// Repeating this on every public primitive would be noise without
// adding information, so the shared clippy doc lints are suppressed
// at crate level and the contract lives in one place.
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]

use winnow::Partial;

pub mod blank;
pub mod boolean;
pub mod iri;
pub mod lang;
pub mod numeric;
pub mod pn;
pub mod string;
pub mod triple_term;
pub mod utf8;
pub mod whitespace;

pub use blank::blank_node_label;
pub use boolean::boolean_literal;
pub use iri::{absolute_iri_ref, iri_ref, is_absolute_iri};
pub use lang::{LangDirection, LangTag, lang_tag};
pub use numeric::{NumericKind, NumericLiteral, numeric_literal};
pub use pn::{pn_chars, pn_chars_base, pn_chars_u, pn_local, pn_prefix};
pub use string::{
    string_literal_long_quote, string_literal_long_single_quote, string_literal_quote,
    string_literal_single_quote,
};
pub use triple_term::{triple_term_close, triple_term_open};
pub use whitespace::{comment, whitespace, whitespace_or_comment};

/// Partial byte-slice input type used by every primitive in this
/// crate. `Partial` makes `take_while` / `take_till` / literal
/// matchers return `ErrMode::Incomplete` rather than `Err` when the
/// buffer ends mid-production, so an asynchronous feeder can resume cleanly
/// on the next chunk.
pub type Input<'a> = Partial<&'a [u8]>;

/// Shorthand for the `ModalResult` type used across the crate.
///
/// All primitives use winnow's default `ContextError<StrContext>`,
/// which carries a `StrContext::Label("production_name")` attached
/// at the entry point of each primitive plus any `StrContext::Expected`
/// annotations added by inner combinators.
pub type RdfResult<T> = winnow::ModalResult<T>;
