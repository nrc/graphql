// Parses an IDL representation of a schema.

use {ParseError, QlResult, QlError};
use parser::lexer::tokenise;
use parser::token::{Atom, Bracket, Token, TokenKind};
use schema::{Field, Item, Interface, Object, Enum, Type, Schema};
use types::Name;

use std::collections::HashMap;

pub fn parse_schema(input: &str) -> QlResult<Schema> {
    let tokens = tokenise(input.trim())?;
    let mut parser = Parser::new(&tokens)?;
    parser.parse_doc()
}

struct Parser<'a> {
    tokens: &'a [Token<'a>],
}

// QUESTION can we share more code with the query parser?
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
        while let Some(tok) = self.tokens.get(0) {
            match tok.kind {
                TokenKind::Atom(Atom::NewLine) => self.bump(),
                _ => return,
            }
        }
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

    fn expect<F, T>(&mut self, f: F) -> QlResult<T>
    where
        F: Fn(&mut Self) -> QlResult<Option<T>>
    {
        f(self).and_then(|n| n.ok_or_else(|| QlError::ParseError(ParseError("Unexpected eof"))))
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

    fn parse_doc(&mut self) -> QlResult<Schema> {
        self.ignore_newlines();
        let mut items = HashMap::new();
        while let Some((name, item)) = self.maybe_parse_item()? {
            self.ignore_newlines();
            items.insert(name, item);
        }
        Ok(Schema { items })
        // TODO check there are no more tokens
    }

    fn maybe_parse_item(&mut self) -> QlResult<Option<(Name, Item)>> {
        let kw = none_ok!(self.maybe_parse_name()?);

        if kw.0 == KSchema::TEXT {
            let body = self.parse_interface()?;
            let item = Item::Schema(body);
            return Ok(Some((Name("schema".to_owned()), item)));
        }

        let name = self.expect(Self::maybe_parse_name)?;

        let item = match &*kw.0 {
            KInterface::TEXT => {
                let body = self.parse_interface()?;
                Item::Interface(body)
            }
            KType::TEXT => {
                let body = self.parse_object()?;
                Item::Object(body)
            }
            KEnum::TEXT => {
                let body = self.parse_enum()?;
                Item::Enum(body)
            }
            _ => return parse_err!("Unexpected item"),
        };

        Ok(Some((name, item)))
    }

    fn parse_interface(&mut self) -> QlResult<Interface> {
        let fields = match self.next_tok()?.kind {
            TokenKind::Tree(Bracket::Brace, ref toks) => {
                Parser::new(toks)?.parse_list(Self::maybe_parse_field)?
            }
            _ => return parse_err!("Expected `{`"),
        };
        Ok(Interface {
            fields,
        })
    }

    fn parse_object(&mut self) -> QlResult<Object> {
        let implements = match self.peek_tok() {
            Some(tok) => match tok.kind {
                TokenKind::Atom(Atom::Name(KImplements::TEXT)) => {
                    self.bump();
                    let name = self.expect(Self::maybe_parse_name)?;
                    // TODO handle a list of names, not just one.
                    vec![name]
                }
                _ => vec![],
            }
            None => vec![],
        };
        let fields = match self.next_tok()?.kind {
            TokenKind::Tree(Bracket::Brace, ref toks) => {
                Parser::new(toks)?.parse_list(Self::maybe_parse_field)?
            }
            _ => return parse_err!("Expected `{`"),
        };
        Ok(Object {
            implements,
            fields,
        })
    }

    fn parse_enum(&mut self) -> QlResult<Enum> {
        let variants = match self.next_tok()?.kind {
            TokenKind::Tree(Bracket::Brace, ref toks) => {
                Parser::new(toks)?.parse_list(Self::maybe_parse_variant)?
            }
            _ => return parse_err!("Expected `{`"),
        };
        Ok(Enum {
            variants,
        })
    }

    fn maybe_parse_field(&mut self) -> QlResult<Option<Field>> {
        let name = none_ok!(self.maybe_parse_name()?);
        let args = if let Some(&Token { kind: TokenKind::Tree(Bracket::Paren, ref toks)}) = self.peek_tok() {
            self.bump();
            Parser::new(toks)?.parse_list(Self::maybe_parse_arg)?
        } else {
            vec![]
        };
        self.eat(Atom::Colon)?;
        let ty = self.parse_type()?;
        Ok(Some(Field {
            name,
            args,
            ty,
        }))
    }

    fn maybe_parse_variant(&mut self) -> QlResult<Option<Name>> {
        let name = none_ok!(self.maybe_parse_name()?);
        Ok(Some(name))
    }

    // Name : Type
    fn maybe_parse_arg(&mut self) -> QlResult<Option<(Name, Type)>> {
        let name = none_ok!(self.maybe_parse_name()?);
        self.eat(Atom::Colon)?;
        let ty = self.parse_type()?;
        Ok(Some((name, ty)))
    }

    // T ::= "String" | "ID" | Name | [T*] | T!
    fn parse_type(&mut self) -> QlResult<Type> {
        let mut result = match self.next_tok()?.kind {
            TokenKind::Tree(Bracket::Square, ref toks) => {
                Type::Array(Box::new(Parser::new(toks)?.parse_type()?))
            }
            TokenKind::Atom(Atom::Name(s)) => {
                match s {
                    KString::TEXT => Type::String,
                    KId::TEXT => Type::Id,
                    _ => Type::Name(Name(s.to_owned())),
                }
            }
            _ => return parse_err!("Unexpected token, expected type"),
        };

        // Looks for `!`s (non-null types).
        loop {
            match self.peek_tok() {
                None => break,
                Some(tok) => {
                    match tok.kind {
                        TokenKind::Atom(Atom::Bang) => {
                            self.bump();
                            result = Type::NonNull(Box::new(result));
                        }
                        _ => break,
                    }
                }
            }
        }

        Ok(result)
    }
}

trait Keyword {
    const TEXT: &'static str;
}

struct KSchema;
struct KType;
struct KEnum;
struct KInterface;
struct KImplements;
struct KId;
struct KString;

impl Keyword for KSchema {
    const TEXT: &'static str = "schema";
}
impl Keyword for KType {
    const TEXT: &'static str = "type";
}
impl Keyword for KEnum {
    const TEXT: &'static str = "enum";
}
impl Keyword for KInterface {
    const TEXT: &'static str = "interface";
}
impl Keyword for KImplements {
    const TEXT: &'static str = "implements";
}
impl Keyword for KString {
    const TEXT: &'static str = "String";
}
impl Keyword for KId {
    const TEXT: &'static str = "ID";
}

macro parse_err($s: expr) {
    Err(QlError::ParseError(ParseError($s)))
}

macro none_ok($e: expr) {
    match $e {
        Some(tok) => tok,
        None => return Ok(None),
    }
}


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_type() {
        let tokens = tokenise("foo!").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        let result = parser.parse_type().unwrap();
        match result {
            Type::NonNull(inner) => match *inner {
                Type::Name(n) => assert_eq!(n.0, "foo"),
                _ => panic!(),
            }
            _ => panic!(),
        }

        let tokens = tokenise("[ID!]!").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        let result = parser.parse_type().unwrap();
        match result {
            Type::NonNull(inner) => match *inner {
                Type::Array(inner) => match *inner {
                    Type::NonNull(inner) => match *inner {
                        Type::Id => {}
                        _ => panic!(),
                    }
                    _ => panic!(),
                },
                _ => panic!(),
            }
            _ => panic!(),
        }
    }

    #[test]
    fn test_parse_doc() {
        let tokens = tokenise(r"
            schema {
                query: Query
            }

            type Query {
                hero(episode: Episode): Character
                human(id : ID!): Human
            }

            enum Episode {
                NEWHOPE
                EMPIRE
                JEDI
            }

            interface Character {
                id: ID!
                name: String!
                friends: [Character]
                appearsIn: [Episode]!
            }

            type Human implements Character {
                id: ID!
                name: String!
                friends: [Character]
                appearsIn: [Episode]!
                homePlanet: String
            }
        ").unwrap();
        let mut parser = Parser::new(&tokens).unwrap();
        let result = parser.parse_doc().unwrap();
        assert_eq!(result.items.len(), 5);
        let schema = &result.items[&Name("schema".to_owned())];
        match *schema {
            Item::Schema(ref i) => {
                assert_eq!(i.fields.len(), 1);
            }
            _ => panic!(),
        }
        let epsiode = &result.items[&Name("Episode".to_owned())];
        match *epsiode {
            Item::Enum(ref e) => {
                assert_eq!(e.variants.len(), 3);
                assert_eq!(e.variants[0].0, "NEWHOPE");
                assert_eq!(e.variants[1].0, "EMPIRE");
                assert_eq!(e.variants[2].0, "JEDI");
            }
            _ => panic!(),
        }
        let human = &result.items[&Name("Human".to_owned())];
        match *human {
            Item::Object(ref o) => {
                assert_eq!(o.implements.len(), 1);
                assert_eq!(o.implements[0].0, "Character");
                assert_eq!(o.fields.len(), 5);
                assert_eq!(o.fields[0].name.0, "id");
                assert_eq!(o.fields[3].name.0, "appearsIn");
                let ep_ty = &o.fields[3].ty;
                match *ep_ty {
                    Type::NonNull(ref inner) => match **inner {
                        Type::Array(ref inner) => match **inner {
                            Type::Name(ref inner) => assert_eq!(inner.0, "Episode"),
                            _ => panic!(),
                        },
                        _ => panic!(),
                    }
                    _ => panic!(),
                }                
            }
            _ => panic!(),
        }
    }
}
