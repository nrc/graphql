#![feature(decl_macro)]
#![feature(associated_type_defaults)]

extern crate rls_span;

use std::error::Error;
use types::{query, result, schema};

pub mod types;
pub mod execution;
mod lexer;
mod parse_query;

pub type QlResult<T> = Result<T, QlError>;

// FIXME use Failure
#[derive(Debug)]
pub enum QlError {
    LexError(lexer::LexError),
    ParseError(ParseError),
    ValidationError,
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

pub fn handle_query<R: query::Root>(query: &str, schema: &schema::Schema, root: R) ->QlResult<result::Value> {
    let query = query::Query::parse(query)?;
    query.validate(schema)?;
    query.execute(schema, root)
}

mod test {
    use types::schema::*;
    use types::query::Query;

    //#[test]
    fn smoke() {
        let mut schema = Schema::new();

        schema.items.insert("Episode", Item::Enum(Enum { variants: vec!["NEWHOPE", "EMPIRE", "JEDI"] }));

        let char_fields = vec![
            Field::field("id", Type::non_null(Type::Id)),
            Field::field("name", Type::non_null(Type::String)),
            Field::field("friends", Type::array(Type::Name("Character"))),
            Field::field("appearsIn", Type::non_null(Type::array(Type::Name("Episode")))),
        ];
        schema.items.insert("Character", Item::Interface(Interface { fields: char_fields.clone() }));
        let mut human_fields = char_fields.clone();
        human_fields.push(Field::field("homePlanet", Type::String));
        schema.items.insert("Human", Item::Object(Object { implements: vec!["Character"], fields: human_fields }));
        schema.items.insert("Query", Item::Object(Object { implements: vec![], fields: vec![
            Field::fun("hero", vec![("episode", Type::Name("Episode"))], Type::Name("Character")),
            Field::fun("human", vec![("id", Type::non_null(Type::Id))], Type::Name("Human")),
        ] }));

        let query = Query::parse(
           "{
              human(id: 1002) {
                name,
                appearsIn,
                id
              }
            }").unwrap();
        query.validate(&schema).unwrap();
        // TODO need to use root from example
        //query.execute(&schema, root).unwrap();
    }
}
