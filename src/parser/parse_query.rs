use {ParseError, QlResult, QlError};
use parser::lexer::tokenise;
use parser::token::{Atom, Bracket, Token, TokenKind};
use query::{Field, Query, Value};
use types::Name;

pub fn parse_query(input: &str) -> QlResult<Query> {
    let tokens = tokenise(input.trim())?;
    let mut parser = Parser::new(&tokens)?;
    parser.parse_query()
}

struct Parser<'a> {
    tokens: &'a [Token<'a>],
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a [Token<'a>]) -> QlResult<Parser<'a>> {
        Ok(Parser {
            tokens,
        })
    }

    fn next_tok(&mut self) -> QlResult<&'a Token<'a>> {
        if self.tokens.is_empty() {
            return parse_err!("Unexpected end of stream");
        }
        let result = &self.tokens[0];
        self.bump();
        Ok(result)
    }

    // Precondition: !self.tokens.is_empty()
    fn bump(&mut self) {
        self.tokens = &self.tokens[1..];
    }

    fn peek_tok(&mut self) -> Option<&'a Token<'a>> {
        self.tokens.get(0)
    }

    fn eat(&mut self, atom: Atom<'a>) -> QlResult<()> {
        match self.next_tok()?.kind {
            TokenKind::Atom(a) if a == atom => Ok(()),
            _ => parse_err!("Unexpected token")
        }
    }

    fn maybe_eat(&mut self, atom: Atom<'a>) {
        if let Some(tok) = self.peek_tok() {
            if let TokenKind::Atom(a) = tok.kind {
                if a == atom {
                    self.bump();
                }
            }
        }
    }

    fn ignore_newlines(&mut self) {
        while let Some(tok) = self.peek_tok() {
            match tok.kind {
                TokenKind::Atom(Atom::NewLine) => self.bump(),
                _ => return,
            }
        }
    }

    fn parse_query(&mut self) -> QlResult<Query> {
        match self.next_tok()?.kind {
            // TODO abstract out keywords
            TokenKind::Atom(Atom::Name(n)) if n == "query" => {
                let body = match self.next_tok()?.kind {
                    TokenKind::Tree(Bracket::Brace, ref toks) => {
                        Parser::new(toks)?.parse_field_list()?
                    }
                    _ => return parse_err!("Unexpected token, expected: `{`"),
                };
                Ok(Query::Query(body))
            }
            TokenKind::Atom(Atom::Name(n)) if n == "mutation" => {
                // TODO parse the body of the mutation
                Ok(Query::Mutation)
            }
            TokenKind::Tree(Bracket::Brace, ref toks) => {
                let body = Parser::new(toks)?.parse_field_list()?;
                Ok(Query::Query(body))
            }
            _ => parse_err!("Unexpected token, expected: identifier or `{`"),
        }
        // TODO assert no more tokens
    }


    fn parse_field_list(&mut self) -> QlResult<Vec<Field>> {
        self.parse_list(Self::maybe_parse_field)
    }

    fn parse_arg_list(&mut self) -> QlResult<Vec<(Name, Value)>> {
        self.parse_list(Self::maybe_parse_arg)
    }

    fn parse_value_list(&mut self) -> QlResult<Vec<Value>> {
        self.parse_list(Self::maybe_parse_value)
    }

    fn parse_list<F, T>(&mut self, f: F) -> QlResult<Vec<T>>
    where
        F: Fn(&mut Self) -> QlResult<Option<T>>
    {
        self.ignore_newlines();

        let mut result = vec![];
        while let Some(arg) = f(self)? {
            result.push(arg);
            self.maybe_eat(Atom::Comma);
            self.ignore_newlines();
        }
        
        Ok(result)
    }

    fn parse_name(&mut self) -> QlResult<Name> {
        self.expect(Self::maybe_parse_name)
    }

    fn parse_value(&mut self) -> QlResult<Value> {
        self.expect(Self::maybe_parse_value)
    }

    fn expect<F, T>(&mut self, f: F) -> QlResult<T>
    where
        F: Fn(&mut Self) -> QlResult<Option<T>>
    {
        f(self).and_then(|n| n.ok_or_else(|| QlError::ParseError(ParseError("Unexpected eof"))))
    }

    fn maybe_parse_name(&mut self) -> QlResult<Option<Name>> {
        match *none_ok!(self.peek_tok()) {
            Token { kind: TokenKind::Atom(Atom::Name(s))} => {
                self.bump();
                Ok(Some(Name(s.to_owned())))
            }
            _ => parse_err!("Unexpected token, expected: name"),
        }
    }

    // Name (args)? { field list }?
    fn maybe_parse_field(&mut self) -> QlResult<Option<Field>> {
        let name = none_ok!(self.maybe_parse_name()?);
        let args = self.maybe_parse_args()?;
        let fields = self.maybe_parse_fields()?;

        Ok(Some(Field {
            name,
            alias: None,
            args,
            fields,
        }))
    }

    // Name : Value
    fn maybe_parse_arg(&mut self) -> QlResult<Option<(Name, Value)>> {
        let name = none_ok!(self.maybe_parse_name()?);
        self.eat(Atom::Colon)?;
        let value = self.parse_value()?;
        Ok(Some((name, value)))
    }

    fn maybe_parse_value(&mut self) -> QlResult<Option<Value>> {
        let result = match none_ok!(self.peek_tok()).kind {
            TokenKind::Atom(Atom::Name("null")) => Value::Null,
            TokenKind::Atom(Atom::Name(s)) => Value::Name(Name(s.to_owned())),
            // TODO this is dumb - we parse a string to a number in the tokeniser, then
            // convert it back to a string here. Perhaps we'll add a Number value later?
            // If not we should treat numbers as Names in the tokeniser.
            TokenKind::Atom(Atom::Number(n)) => Value::Name(Name(n.to_string())),
            TokenKind::Atom(Atom::String(s)) => Value::String(s.to_owned()),
            TokenKind::Tree(Bracket::Square, ref toks) => {
                Value::Array(Parser::new(toks)?.parse_value_list()?)
            }
            _ => return parse_err!("Unexpected token, expected: value"),
        };

        self.bump();
        Ok(Some(result))
    }

    fn maybe_parse_args(&mut self) -> QlResult<Vec<(Name, Value)>> {
        self.maybe_parse_seq(Bracket::Paren, Self::parse_arg_list)
    }

    fn maybe_parse_fields(&mut self) -> QlResult<Vec<Field>> {
        self.maybe_parse_seq(Bracket::Brace, Self::parse_field_list)
    }

    fn maybe_parse_seq<F, T>(&mut self, opener: Bracket, f: F) -> QlResult<Vec<T>>
    where
        F: Fn(&mut Self) -> QlResult<Vec<T>>
    {
        if let Some(tok) = self.peek_tok() {
            if let TokenKind::Tree(br, ref toks) = tok.kind {
                if br == opener {
                    self.bump();
                    return f(&mut Parser::new(toks)?);
                }
            }
        }
        Ok(vec![])
    }
}

macro none_ok($e: expr) {
    match $e {
        Some(tok) => tok,
        None => return Ok(None),
    }
}

macro parse_err($s: expr) {
    Err(QlError::ParseError(ParseError($s)))
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bump() {
        let tokens = tokenise("foo bar").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.tokens.len(), 2);
        parser.bump();
        assert_eq!(parser.tokens.len(), 1);
        parser.bump();
        assert_eq!(parser.tokens.len(), 0);
    }

    #[test]
    fn test_maybe_eat() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(assert_atom(parser.peek_tok().unwrap()), Atom::Name("foo"));
        parser.maybe_eat(Atom::Name("bar"));
        assert_eq!(assert_atom(parser.next_tok().unwrap()), Atom::Name("foo"));
        parser.maybe_eat(Atom::Name("bar"));
        parser.maybe_eat(Atom::Bang);
    }

    #[test]
    fn test_eat() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(assert_atom(parser.next_tok().unwrap()), Atom::Name("foo"));
        parser.eat(Atom::Name("bar")).unwrap();
        parser.eat(Atom::Bang).unwrap();
    }

    #[test]
    fn test_bad_eat() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        match parser.eat(Atom::Name("bar")) {
            Err(QlError::ParseError(ParseError(_))) => {}
            result => panic!("Found: {:?}", result),
        }
    }

    #[test]
    fn test_ignore_newlines() {
        let tokens = tokenise("foo \n\n\n\n\n bar").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(assert_atom(parser.peek_tok().unwrap()), Atom::Name("foo"));
        parser.ignore_newlines();
        assert_eq!(assert_atom(parser.next_tok().unwrap()), Atom::Name("foo"));
        parser.ignore_newlines();
        assert_eq!(assert_atom(parser.next_tok().unwrap()), Atom::Name("bar"));
    }

    fn assert_atom<'a>(tok: &Token<'a>) -> Atom<'a> {
        match tok.kind {
            TokenKind::Atom(atom) => atom,
            _ => panic!("Non-atomic token"),
        }
    }

    #[test]
    fn test_parse_name() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.parse_name().unwrap().0, "foo");
        assert_eq!(parser.parse_name().unwrap().0, "bar");
        parser.eat(Atom::Bang).unwrap();
    }

    #[test]
    fn test_parse_value() {
        let tokens = tokenise("null \"foo\" 42 bar [null, null, foo, \"bar\"]").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.parse_value().unwrap(), Value::Null);
        assert_eq!(parser.parse_value().unwrap(), Value::String("foo".to_owned()));
        assert_eq!(parser.parse_value().unwrap(), Value::Name(Name("42".to_owned())));
        assert_eq!(parser.parse_value().unwrap(), Value::Name(Name("bar".to_owned())));
        assert_eq!(parser.parse_value().unwrap(), Value::Array(vec![Value::Null, Value::Null, Value::Name(Name("foo".to_owned())), Value::String("bar".to_owned())]));
    }

    #[test]
    fn test_parse_args() {
        let tokens = tokenise("  ").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.maybe_parse_args().unwrap(), vec![]);

        let tokens = tokenise("(x: 42, foo: \"bar\")").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.maybe_parse_args().unwrap(), vec![(Name("x".to_owned()), Value::Name(Name("42".to_owned()))), (Name("foo".to_owned()), Value::String("bar".to_owned()))]);
    }

    #[test]
    fn test_parse_fields() {
        let tokens = tokenise("").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.maybe_parse_fields().unwrap(), vec![]);

        let tokens = tokenise("{}").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.maybe_parse_fields().unwrap(), vec![]);

        fn name_field(s: &str) -> Field {
            Field {
                name: Name(s.to_owned()),
                alias: None,
                args: vec![],
                fields: vec![],
            }
        }

        let tokens = tokenise(r"{ a, foo, bar(x: 42)

            baz {
                a
                b
            }}").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        assert_eq!(parser.maybe_parse_fields().unwrap(), vec![
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
        ]);
    }

    #[test]
    fn test_parse_query() {
        let tokens = tokenise(r"{
          human(id: 1002) {
            name,
            appearsIn,
            id
          }
        }").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        let result = parser.parse_query().unwrap();
        if let Query::Query(fields) = result {
            assert_eq!(fields.len(), 1);
            assert_eq!(&*fields[0].name.0, "human");
            assert_eq!(fields[0].args.len(), 1);
            assert_eq!(&fields[0].args[0], &(Name("id".to_owned()), Value::Name(Name("1002".to_owned()))));
            assert_eq!(fields[0].fields.len(), 3);
            assert_eq!(fields[0].fields[0].name.0, "name");
            assert_eq!(fields[0].fields[1].name.0, "appearsIn");
            assert_eq!(fields[0].fields[2].name.0, "id");
        } else {
            panic!();
        }
    }
}
