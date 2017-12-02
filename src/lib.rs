#![feature(decl_macro)]
#![feature(associated_type_defaults)]

// Big TODOs
//
// refactoring in query parser
// schema validation
// schema parser
// macro-ification (and refactoring in example)
// end to end test!
// 0.1
// validation caching

extern crate rls_span;

use std::error::Error;
use types::{query, result, schema};

pub mod execution;
mod parser;
pub mod types;
pub mod validation;

pub type QlResult<T> = Result<T, QlError>;

// FIXME use Failure
#[derive(Debug)]
pub enum QlError {
    LexError(parser::lexer::LexError),
    ParseError(ParseError),
    ValidationError(Vec<validation::Error>),
    ExecutionError(String),
    // (from, to) TODO - TranslationError
    LoweringError(String, String),
    // (kind, input, expected)
    ResolveError(&'static str, String, Option<String>),
    // An Error in user code.
    ServerError(Box<Error>),
}

#[derive(Debug)]
pub struct ParseError(&'static str);

pub fn handle_query<R: query::Root>(query: &str, schema: &schema::Schema, root: R) -> QlResult<result::Value> {
    let query = query::Query::parse(query)?;
    query.validate(schema)?;
    query.execute(schema, root)
}
