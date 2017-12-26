// Parses an IDL representation of a schema.

use {ParseError, QlResult, QlError};
use parser::lexer::tokenise;
use parser::parse_base::{none_ok, parse_err, TokenStream, maybe_parse_name};
use parser::token::{Atom, Bracket, Token, TokenKind};
use schema::{Field, Item, Interface, Object, Enum, Type, TypeKind, Schema};
use types::Name;

use std::collections::HashMap;

pub fn parse_schema(input: &str) -> QlResult<Schema> {
    let tokens = tokenise(input.trim())?;
    let mut stream = TokenStream::new(&tokens);
    parse_doc(&mut stream)
}

fn parse_doc(stream: &mut TokenStream) -> QlResult<Schema> {
    stream.ignore_newlines();
    let mut items = HashMap::new();
    while let Some((name, item)) = maybe_parse_item(stream)? {
        stream.ignore_newlines();
        items.insert(name, item);
    }
    Ok(Schema { items })
    // TODO check there are no more tokens
}

fn maybe_parse_item(stream: &mut TokenStream) -> QlResult<Option<(Name, Item)>> {
    let kw = none_ok!(maybe_parse_name(stream)?);

    if kw.0 == KSchema::TEXT {
        let body = parse_interface(stream)?;
        let item = Item::Schema(body);
        return Ok(Some((Name("schema".to_owned()), item)));
    }

    let name = stream.expect(maybe_parse_name)?;

    let item = match &*kw.0 {
        KInterface::TEXT => {
            let body = parse_interface(stream)?;
            Item::Interface(body)
        }
        KType::TEXT => {
            let body = parse_object(stream)?;
            Item::Object(body)
        }
        KEnum::TEXT => {
            let body = parse_enum(stream)?;
            Item::Enum(body)
        }
        _ => return parse_err!("Unexpected item"),
    };

    Ok(Some((name, item)))
}

fn parse_interface(stream: &mut TokenStream) -> QlResult<Interface> {
    let fields = match stream.next_tok()?.kind {
        TokenKind::Tree(Bracket::Brace, ref toks) => {
            TokenStream::new(toks).parse_list(maybe_parse_field)?
        }
        _ => return parse_err!("Expected `{`"),
    };
    Ok(Interface {
        fields,
    })
}

fn parse_object(stream: &mut TokenStream) -> QlResult<Object> {
    let implements = match stream.peek_tok() {
        Some(tok) => match tok.kind {
            TokenKind::Atom(Atom::Name(KImplements::TEXT)) => {
                stream.bump();
                let name = stream.expect(maybe_parse_name)?;
                // TODO handle a list of names, not just one.
                vec![name]
            }
            _ => vec![],
        }
        None => vec![],
    };
    let fields = match stream.next_tok()?.kind {
        TokenKind::Tree(Bracket::Brace, ref toks) => {
            TokenStream::new(toks).parse_list(maybe_parse_field)?
        }
        _ => return parse_err!("Expected `{`"),
    };
    Ok(Object {
        implements,
        fields,
    })
}

fn parse_enum(stream: &mut TokenStream) -> QlResult<Enum> {
    let variants = match stream.next_tok()?.kind {
        TokenKind::Tree(Bracket::Brace, ref toks) => {
            TokenStream::new(toks).parse_list(maybe_parse_variant)?
        }
        _ => return parse_err!("Expected `{`"),
    };
    Ok(Enum {
        variants,
    })
}

fn maybe_parse_field(stream: &mut TokenStream) -> QlResult<Option<Field>> {
    let name = none_ok!(maybe_parse_name(stream)?);
    let args = if let Some(&Token { kind: TokenKind::Tree(Bracket::Paren, ref toks)}) = stream.peek_tok() {
        stream.bump();
        TokenStream::new(toks).parse_list(maybe_parse_arg)?
    } else {
        vec![]
    };
    stream.eat(Atom::Colon)?;
    let ty = parse_type(stream)?;
    Ok(Some(Field {
        name,
        args,
        ty,
    }))
}

fn maybe_parse_variant(stream: &mut TokenStream) -> QlResult<Option<Name>> {
    let name = none_ok!(maybe_parse_name(stream)?);
    Ok(Some(name))
}

// Name : Type
fn maybe_parse_arg(stream: &mut TokenStream) -> QlResult<Option<(Name, Type)>> {
    let name = none_ok!(maybe_parse_name(stream)?);
    stream.eat(Atom::Colon)?;
    let ty = parse_type(stream)?;
    Ok(Some((name, ty)))
}

// T ::= "String" | "ID" | Name | [T*] | T!
fn parse_type(stream: &mut TokenStream) -> QlResult<Type> {
    let mut result = match stream.next_tok()?.kind {
        TokenKind::Tree(Bracket::Square, ref toks) => {
            Type::array(parse_type(&mut TokenStream::new(toks))?)
        }
        TokenKind::Atom(Atom::Name(s)) => {
            Type {
                kind: match s {
                    KString::TEXT => TypeKind::String,
                    KId::TEXT => TypeKind::Id,
                    _ => TypeKind::Name(Name(s.to_owned())),
                },
                nullable: true,
            }
        }
        _ => return parse_err!("Unexpected token, expected type"),
    };

    // Looks for `!`s (non-null types).
    loop {
        match stream.peek_tok() {
            None => break,
            Some(tok) => {
                match tok.kind {
                    TokenKind::Atom(Atom::Bang) => {
                        stream.bump();
                        result.nullable = false;
                    }
                    _ => break,
                }
            }
        }
    }

    Ok(result)
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


#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_parse_type() {
        let tokens = tokenise("foo").unwrap();
        let mut ts = TokenStream::new(&tokens);
        let result = parse_type(&mut ts).unwrap();
        assert_eq!(result.nullable, true);
        match result.kind {
            TypeKind::Name(n) => assert_eq!(n.0, "foo"),
            _ => panic!(),
        }

        let tokens = tokenise("[ID!]!").unwrap();
        let mut ts = TokenStream::new(&tokens);
        let result = parse_type(&mut ts).unwrap();
        assert_eq!(result.nullable, false);
        match result.kind {
            TypeKind::Array(inner) => {
                assert_eq!(inner.nullable, false);
                match inner.kind {
                    TypeKind::Id => {}
                    _ => panic!(),
                }
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
        let mut ts = TokenStream::new(&tokens);
        let result = parse_doc(&mut ts).unwrap();
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
                match ep_ty.kind {
                    TypeKind::Array(ref inner) => match inner.kind {
                        TypeKind::Name(ref inner) => assert_eq!(inner.0, "Episode"),
                        _ => panic!(),
                    },
                    _ => panic!(),
                }                
            }
            _ => panic!(),
        }
    }
}
