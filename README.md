# rdf-winnow

[![Crates.io](https://img.shields.io/crates/v/rdf-winnow.svg)](https://crates.io/crates/rdf-winnow)
[![Documentation](https://docs.rs/rdf-winnow/badge.svg)](https://docs.rs/rdf-winnow)
[![CI](https://github.com/legra-ai/rdf-winnow/actions/workflows/ci.yml/badge.svg)](https://github.com/legra-ai/rdf-winnow/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/rdf-winnow.svg)](https://github.com/legra-ai/rdf-winnow/blob/main/LICENSE-APACHE)
[![Downloads](https://img.shields.io/crates/d/rdf-winnow.svg)](https://crates.io/crates/rdf-winnow)

Streaming RDF lexical primitives built on [`winnow`](https://docs.rs/winnow).
The crate parses small RDF grammar productions from partial byte slices so
format-specific parsers can handle input one chunk at a time without losing
the distinction between incomplete input and invalid syntax.

## Scope

`rdf-winnow` provides the reusable lexical layer used by text RDF parsers:

- IRI references and absolute IRI references;
- blank-node labels;
- language tags, including RDF directional language tags;
- numeric and boolean literals;
- quoted string forms and escapes;
- prefixed-name components;
- RDF-star/triple-term delimiters;
- UTF-8 characters, whitespace, and comments.

It does not parse a complete RDF document, build a graph, perform IRI
resolution, or assign application semantics to a datatype. A caller composes
these primitives into Turtle, `TriG`, N-Triples, N-Quads, or another RDF
syntax.

## Partial input

Every parser consumes a `winnow::Partial<&[u8]>`. A parser returns
`ErrMode::Incomplete` when the current buffer ends in the middle of a valid
production. It returns a contextual syntax error when the input cannot be
valid. This distinction lets an asynchronous feeder read another chunk only
when more bytes are actually needed.

```rust
use rdf_winnow::{iri_ref, string_literal_quote, Input, RdfResult};
use winnow::Partial;

fn parse_pair<'a>(input: &mut Input<'a>) -> RdfResult<(&'a [u8], &'a [u8])> {
    let iri = iri_ref(input)?;
    let literal = string_literal_quote(input)?;
    Ok((iri, literal))
}

let mut input = Partial::new(&b"<https://example.org/name>\"Alice\""[..]);
let (iri, literal) = parse_pair(&mut input)?;
assert_eq!(iri, b"https://example.org/name");
assert_eq!(literal, b"Alice");
# Ok::<(), winnow::error::ErrMode<winnow::error::ContextError>>(())
```

The returned byte slices borrow from the caller's input. That keeps the
lexical layer allocation-free for callers that can process each token before
the next buffer is read.

## Typed literals and custom datatypes

RDF permits a literal to use any valid datatype IRI. This crate provides the
IRI and string primitives needed to parse that syntax but does not maintain a
datatype registry or restrict callers to XML Schema datatypes. A higher-level
parser can therefore preserve application-specific datatypes, including
media-oriented datatypes, without changing this crate.

## API overview

| Item | Purpose |
| --- | --- |
| `Input<'a>` | Partial byte-slice input shared by the parsers. |
| `RdfResult<T>` | Winnow result type used by the public primitives. |
| `iri_ref` | Parses an RDF IRI reference body. |
| `absolute_iri_ref` | Parses an IRI reference and requires a scheme. |
| `string_literal_*` | Parses the four RDF quoted-string forms. |
| `lang_tag` | Parses a language tag and optional text direction. |
| `numeric_literal` | Parses RDF numeric lexical forms. |
| `pn_*` | Parses prefixed-name components. |

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.
