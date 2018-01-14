#![feature(decl_macro)]
#![feature(associated_type_defaults)]
#![feature(attr_literals)]

extern crate rls_span;
#[macro_use]
extern crate failure;

use types::{query, result, schema};

pub mod execution;
mod parser;
pub mod types;
pub mod validation;

pub use parser::parse_idl::parse_schema;

pub type QlResult<T> = Result<T, QlError>;

#[derive(Debug, Fail)]
pub enum QlError {
    #[fail(display = "Parsing error: {}", 0)]
    LexError(parser::lexer::LexError),
    #[fail(display = "Parsing error: {:?}", 0)]
    ParseError(parser::ParseError),
    #[fail(display = "Validation error: {:?}", 0)]
    ValidationError(Vec<validation::Error>),
    #[fail(display = "Execution error: {}", 0)]
    ExecutionError(String),
    // (from, to)
    #[fail(display = "Translation error: from {} to {}", 0, 1)]
    TranslationError(String, String),
    // (kind, input, expected)
    #[fail(display = "Could not find {}: Found: {}, expected: {:?}", 0, 1, 2)]
    ResolveError(&'static str, String, Option<String>),
}

pub fn handle_query<R: query::Root>(query: &str, root: R) -> QlResult<result::Value> {
    let schema = &R::schema();
    let query = query::Query::parse(query)?;
    query.validate(schema)?;
    query.execute(schema, root)
}
