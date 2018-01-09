// Code which is shared between the IDL and query parsers

use {ParseError, QlError, QlResult};
use parser::lexer::tokenise;
use parser::token::{Atom, Bracket, Token, TokenKind};
use schema::{Enum, Field, Interface, Item, Object, Schema, Type};
use types::Name;

pub struct TokenStream<'a> {
    tokens: &'a [Token<'a>],
}

impl<'a> TokenStream<'a> {
    pub fn new(tokens: &'a [Token<'a>]) -> TokenStream<'a> {
        TokenStream { tokens }
    }

    pub fn next_tok(&mut self) -> QlResult<&'a Token<'a>> {
        if self.tokens.is_empty() {
            return parse_err!("Unexpected end of stream");
        }
        let result = &self.tokens[0];
        self.bump();
        Ok(result)
    }

    // Precondition: !self.tokens.is_empty()
    pub fn bump(&mut self) {
        self.tokens = &self.tokens[1..];
    }

    pub fn peek_tok(&mut self) -> Option<&'a Token<'a>> {
        self.tokens.get(0)
    }

    pub fn eat(&mut self, atom: Atom<'a>) -> QlResult<()> {
        match self.next_tok()?.kind {
            TokenKind::Atom(a) if a == atom => Ok(()),
            _ => parse_err!("Unexpected token"),
        }
    }

    pub fn maybe_eat(&mut self, atom: Atom<'a>) {
        if let Some(tok) = self.peek_tok() {
            if let TokenKind::Atom(a) = tok.kind {
                if a == atom {
                    self.bump();
                }
            }
        }
    }

    pub fn ignore_newlines(&mut self) {
        while let Some(tok) = self.tokens.get(0) {
            match tok.kind {
                TokenKind::Atom(Atom::NewLine) => self.bump(),
                _ => return,
            }
        }
    }

    pub fn expect<F, T>(&mut self, f: F) -> QlResult<T>
    where
        F: Fn(&mut TokenStream) -> QlResult<Option<T>>,
    {
        f(self).and_then(|n| n.ok_or_else(|| QlError::ParseError(ParseError("Unexpected eof"))))
    }

    pub fn parse_list<F, T>(&mut self, f: F) -> QlResult<Vec<T>>
    where
        F: Fn(&mut TokenStream) -> QlResult<Option<T>>,
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

    pub fn maybe_parse_seq<F, T>(&mut self, opener: Bracket, f: F) -> QlResult<Vec<T>>
    where
        F: Fn(&mut TokenStream) -> QlResult<Vec<T>>,
    {
        if let Some(tok) = self.peek_tok() {
            if let TokenKind::Tree(br, ref toks) = tok.kind {
                if br == opener {
                    self.bump();
                    return f(&mut TokenStream::new(toks));
                }
            }
        }
        Ok(vec![])
    }
}

pub fn maybe_parse_name(stream: &mut TokenStream) -> QlResult<Option<Name>> {
    match *none_ok!(stream.peek_tok()) {
        Token {
            kind: TokenKind::Atom(Atom::Name(s)),
        } => {
            stream.bump();
            Ok(Some(Name(s.to_owned())))
        }
        _ => parse_err!("Unexpected token, expected: name"),
    }
}

pub macro parse_err($s: expr) {
    Err(QlError::ParseError(ParseError($s)))
}

pub macro none_ok($e: expr) {
    match $e {
        Some(tok) => tok,
        None => return Ok(None),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bump() {
        let tokens = tokenise("foo bar").unwrap();
        let mut parser = TokenStream::new(&tokens);
        assert_eq!(parser.tokens.len(), 2);
        parser.bump();
        assert_eq!(parser.tokens.len(), 1);
        parser.bump();
        assert_eq!(parser.tokens.len(), 0);
    }

    #[test]
    fn test_maybe_eat() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = TokenStream::new(&tokens);
        assert_eq!(assert_atom(parser.peek_tok().unwrap()), Atom::Name("foo"));
        parser.maybe_eat(Atom::Name("bar"));
        assert_eq!(assert_atom(parser.next_tok().unwrap()), Atom::Name("foo"));
        parser.maybe_eat(Atom::Name("bar"));
        parser.maybe_eat(Atom::Bang);
    }

    #[test]
    fn test_eat() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = TokenStream::new(&tokens);
        assert_eq!(assert_atom(parser.next_tok().unwrap()), Atom::Name("foo"));
        parser.eat(Atom::Name("bar")).unwrap();
        parser.eat(Atom::Bang).unwrap();
    }

    #[test]
    fn test_bad_eat() {
        let tokens = tokenise("foo bar!").unwrap();
        let mut parser = TokenStream::new(&tokens);
        match parser.eat(Atom::Name("bar")) {
            Err(QlError::ParseError(ParseError(_))) => {}
            result => panic!("Found: {:?}", result),
        }
    }

    #[test]
    fn test_ignore_newlines() {
        let tokens = tokenise("foo \n\n\n\n\n bar").unwrap();
        let mut parser = TokenStream::new(&tokens);
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
}
