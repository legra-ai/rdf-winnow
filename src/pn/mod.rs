//! Prefixed-name (PN) primitives shared between Turtle/TriG and
//! blank-node labels.
//!
//! These implement the W3C Turtle 1.2 `PN_CHARS_BASE`, `PN_CHARS_U`,
//! `PN_CHARS`, `PN_PREFIX`, and `PN_LOCAL` productions. Because
//! several code-point ranges are outside ASCII, they operate on
//! decoded `char`s via [`crate::utf8::utf8_char`] rather than on raw
//! bytes.
//!
//! ```text
//! PN_CHARS_BASE ::= [A-Z] | [a-z]
//!                 | [#x00C0-#x00D6] | [#x00D8-#x00F6] | [#x00F8-#x02FF]
//!                 | [#x0370-#x037D] | [#x037F-#x1FFF]
//!                 | [#x200C-#x200D] | [#x2070-#x218F] | [#x2C00-#x2FEF]
//!                 | [#x3001-#xD7FF] | [#xF900-#xFDCF] | [#xFDF0-#xFFFD]
//!                 | [#x10000-#xEFFFF]
//! PN_CHARS_U    ::= PN_CHARS_BASE | '_'
//! PN_CHARS      ::= PN_CHARS_U | '-' | [0-9] | #x00B7
//!                 | [#x0300-#x036F] | [#x203F-#x2040]
//! PN_PREFIX     ::= PN_CHARS_BASE ((PN_CHARS | '.')* PN_CHARS)?
//! PN_LOCAL      ::= (PN_CHARS_U | ':' | [0-9] | PLX)
//!                   ((PN_CHARS | '.' | ':' | PLX)* (PN_CHARS | ':' | PLX))?
//! PLX           ::= PERCENT | PN_LOCAL_ESC
//! PERCENT       ::= '%' HEX HEX
//! PN_LOCAL_ESC  ::= '\' [_~.!$&'()*+,;=/?#@%-]
//! ```

mod chars;
mod classes;
mod escape;
mod names;
mod util;

#[cfg(test)]
mod tests;

pub use chars::{pn_chars, pn_chars_base, pn_chars_u};
pub use classes::{is_pn_chars, is_pn_chars_base, is_pn_chars_u};
pub use names::{pn_local, pn_prefix};
