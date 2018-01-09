use {ParseError, QlError, QlResult};
use parser::lexer::tokenise;
use parser::parse_base::{maybe_parse_name, none_ok, parse_err, TokenStream};
use parser::token::{Atom, Bracket, Token, TokenKind};
use query::{Field, Query, Value};
use types::Name;

pub fn parse_query(input: &str) -> QlResult<Query> {
    let tokens = tokenise(input.trim())?;
    let mut stream = TokenStream::new(&tokens);
    parse_operation(&mut stream)
}

fn parse_operation(stream: &mut TokenStream) -> QlResult<Query> {
    match stream.next_tok()?.kind {
        // TODO abstract out keywords
        TokenKind::Atom(Atom::Name(n)) if n == "query" => {
            let body = match stream.next_tok()?.kind {
                TokenKind::Tree(Bracket::Brace, ref toks) => {
                    parse_field_list(&mut TokenStream::new(toks))?
                }
                _ => return parse_err!("Unexpected token, expected: `{`"),
            };
            Ok(Query::Query(vec![
                Field {
                    name: Name("query".to_owned()),
                    alias: None,
                    args: vec![],
                    fields: body,
                },
            ]))
        }
        TokenKind::Atom(Atom::Name(n)) if n == "mutation" => {
            // TODO parse the body of the mutation
            Ok(Query::Mutation)
        }
        TokenKind::Tree(Bracket::Brace, ref toks) => {
            let body = parse_field_list(&mut TokenStream::new(toks))?;
            Ok(Query::Query(vec![
                Field {
                    name: Name("query".to_owned()),
                    alias: None,
                    args: vec![],
                    fields: body,
                },
            ]))
        }
        _ => parse_err!("Unexpected token, expected: identifier or `{`"),
    }
    // TODO assert no more tokens
}

fn parse_field_list(stream: &mut TokenStream) -> QlResult<Vec<Field>> {
    stream.parse_list(maybe_parse_field)
}

fn parse_arg_list(stream: &mut TokenStream) -> QlResult<Vec<(Name, Value)>> {
    stream.parse_list(maybe_parse_arg)
}

fn parse_value_list(stream: &mut TokenStream) -> QlResult<Vec<Value>> {
    stream.parse_list(maybe_parse_value)
}

fn parse_name(stream: &mut TokenStream) -> QlResult<Name> {
    stream.expect(maybe_parse_name)
}

fn parse_value(stream: &mut TokenStream) -> QlResult<Value> {
    stream.expect(maybe_parse_value)
}

// Name (args)? { field list }?
fn maybe_parse_field(stream: &mut TokenStream) -> QlResult<Option<Field>> {
    let name = none_ok!(maybe_parse_name(stream)?);
    let args = maybe_parse_args(stream)?;
    let fields = maybe_parse_fields(stream)?;

    Ok(Some(Field {
        name,
        alias: None,
        args,
        fields,
    }))
}

// Name : Value
fn maybe_parse_arg(stream: &mut TokenStream) -> QlResult<Option<(Name, Value)>> {
    let name = none_ok!(maybe_parse_name(stream)?);
    stream.eat(Atom::Colon)?;
    let value = parse_value(stream)?;
    Ok(Some((name, value)))
}

fn maybe_parse_value(stream: &mut TokenStream) -> QlResult<Option<Value>> {
    let result = match none_ok!(stream.peek_tok()).kind {
        TokenKind::Atom(Atom::Name("null")) => Value::Null,
        TokenKind::Atom(Atom::Name(s)) => Value::Name(Name(s.to_owned())),
        // TODO this is dumb - we parse a string to a number in the tokeniser, then
        // convert it back to a string here. Perhaps we'll add a Number value later?
        // If not we should treat numbers as Names in the tokeniser.
        TokenKind::Atom(Atom::Number(n)) => Value::Name(Name(n.to_string())),
        TokenKind::Atom(Atom::String(s)) => Value::String(s.to_owned()),
        TokenKind::Tree(Bracket::Square, ref toks) => {
            Value::Array(parse_value_list(&mut TokenStream::new(toks))?)
        }
        _ => return parse_err!("Unexpected token, expected: value"),
    };

    stream.bump();
    Ok(Some(result))
}

fn maybe_parse_args(stream: &mut TokenStream) -> QlResult<Vec<(Name, Value)>> {
    stream.maybe_parse_seq(Bracket::Paren, parse_arg_list)
}

fn maybe_parse_fields(stream: &mut TokenStream) -> QlResult<Vec<Field>> {
    stream.maybe_parse_seq(Bracket::Brace, parse_field_list)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_name() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(parse_name(&mut ts).unwrap().0, "foo");
        assert_eq!(parse_name(&mut ts).unwrap().0, "bar");
        ts.eat(Atom::Bang).unwrap();
    }

    #[test]
    fn test_parse_value() {
        let tokens = tokenise("null \"foo\" 42 bar [null, null, foo, \"bar\"]").unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(parse_value(&mut ts).unwrap(), Value::Null);
        assert_eq!(
            parse_value(&mut ts).unwrap(),
            Value::String("foo".to_owned())
        );
        assert_eq!(
            parse_value(&mut ts).unwrap(),
            Value::Name(Name("42".to_owned()))
        );
        assert_eq!(
            parse_value(&mut ts).unwrap(),
            Value::Name(Name("bar".to_owned()))
        );
        assert_eq!(
            parse_value(&mut ts).unwrap(),
            Value::Array(vec![
                Value::Null,
                Value::Null,
                Value::Name(Name("foo".to_owned())),
                Value::String("bar".to_owned()),
            ])
        );
    }

    #[test]
    fn test_parse_args() {
        let tokens = tokenise("  ").unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(maybe_parse_args(&mut ts).unwrap(), vec![]);

        let tokens = tokenise("(x: 42, foo: \"bar\")").unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(
            maybe_parse_args(&mut ts).unwrap(),
            vec![
                (Name("x".to_owned()), Value::Name(Name("42".to_owned()))),
                (Name("foo".to_owned()), Value::String("bar".to_owned())),
            ]
        );
    }

    #[test]
    fn test_parse_fields() {
        let tokens = tokenise("").unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(maybe_parse_fields(&mut ts).unwrap(), vec![]);

        let tokens = tokenise("{}").unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(maybe_parse_fields(&mut ts).unwrap(), vec![]);

        fn name_field(s: &str) -> Field {
            Field {
                name: Name(s.to_owned()),
                alias: None,
                args: vec![],
                fields: vec![],
            }
        }

        let tokens = tokenise(
            r"{ a, foo, bar(x: 42)

            baz {
                a
                b
            }}",
        ).unwrap();
        let mut ts = TokenStream::new(&tokens);
        assert_eq!(
            maybe_parse_fields(&mut ts).unwrap(),
            vec![
                name_field("a"),
                name_field("foo"),
                Field {
                    name: Name("bar".to_owned()),
                    alias: None,
                    args: vec![(Name("x".to_owned()), Value::Name(Name("42".to_owned())))],
                    fields: vec![],
                },
                Field {
                    name: Name("baz".to_owned()),
                    alias: None,
                    args: vec![],
                    fields: vec![name_field("a"), name_field("b")],
                },
            ]
        );
    }

    #[test]
    fn test_parse_query() {
        let tokens = tokenise(
            r"{
          human(id: 1002) {
            name,
            appearsIn,
            id
          }
        }",
        ).unwrap();
        let mut ts = TokenStream::new(&tokens);
        let result = parse_operation(&mut ts).unwrap();
        if let Query::Query(fields) = result {
            assert_eq!(fields.len(), 1);
            println!("{:?}", fields);
            assert_eq!(fields[0].name.0, "query");
            assert_eq!(fields[0].args.len(), 0);
            assert_eq!(fields[0].fields.len(), 1);
            assert_eq!(fields[0].fields[0].name.0, "human");
            assert_eq!(
                &fields[0].fields[0].args[0],
                &(Name("id".to_owned()), Value::Name(Name("1002".to_owned())))
            );
            assert_eq!(fields[0].fields[0].fields.len(), 3);
            assert_eq!(fields[0].fields[0].fields[0].name.0, "name");
            assert_eq!(fields[0].fields[0].fields[1].name.0, "appearsIn");
            assert_eq!(fields[0].fields[0].fields[2].name.0, "id");
        } else {
            panic!();
        }
    }
}
