use {QlError, QlResult};
use parser::token::{Atom, Bracket, Token, TokenKind};

use std::fmt;
use std::iter::Peekable;
use std::mem;
use std::str::{CharIndices, FromStr};

pub fn tokenise<'a>(input: &'a str) -> QlResult<Vec<Token<'a>>> {
    let lexer = Lexer::new(input);
    lexer.tokenise()
}

// pub type Span = ::rls_span::Span<::rls_span::ZeroIndexed>;

// TODO should be able to track multiple errors
#[derive(Clone, Debug)]
// TODO all variants should include info for the error position
pub enum LexError {
    Unexpected(char),
    // TODO could include span info with first thing
    Unmatched(char, char),
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexError::Unexpected(c) => writeln!(f, "Unexpected token: {}", c),
            LexError::Unmatched(expected, found) => writeln!(f, "Unmatched delimiter: expected {}, found {}", expected, found),
        }
    }
}

macro lex_err($kind: ident $(, $body: expr)*) {
    Err(QlError::LexError(LexError::$kind($($body,)*)))
}

struct Lexer<'a> {
    input: &'a str,
    iter: Peekable<CharIndices<'a>>,
    result: Vec<Token<'a>>,
    tree_stack: Vec<(Bracket, Vec<Token<'a>>)>,
    string: Option<usize>,
}

impl<'a> Lexer<'a> {
    fn new(input: &'a str) -> Lexer<'a> {
        Lexer {
            input,
            iter: input.char_indices().peekable(),
            result: vec![],
            tree_stack: vec![],
            string: None,
        }
    }

    fn tokenise(mut self) -> QlResult<Vec<Token<'a>>> {
        while let Some((i, c)) = self.iter.next() {
            // TODO escaped closing quote
            if self.string.is_some() && c != '"' {
                continue;
            }

            match c {
                '#' => self.comment(i),
                '{' => self.open_bracket(Bracket::Brace),
                br @ '}' | br @ ']' | br @ ')' => self.close_bracket(br)?,
                '[' => self.open_bracket(Bracket::Square),
                '(' => self.open_bracket(Bracket::Paren),

                '"' => match self.string {
                    Some(_) => self.close_string(i),
                    None => self.open_string(i),
                },

                '\n' => {
                    // TODO span stuff
                    self.atom(Atom::NewLine);
                }
                '!' => self.atom(Atom::Bang),
                ':' => self.atom(Atom::Colon),
                ',' => self.atom(Atom::Comma),

                '-' => self.number(i),
                c if c.is_digit(10) => self.number(i),

                c if c.is_alphabetic() => self.name(i),

                // This includes \r
                c if c.is_whitespace() => {}

                c => return lex_err!(Unexpected, c),
            }
        }

        for _tree in &self.tree_stack {
            // TODO each tree is an unclosed delimiter error
        }

        if let Some(_s) = self.string {
            // TODO unclosed string
        }

        Ok(self.result)
    }

    fn atom(&mut self, atom: Atom<'a>) {
        self.result.push(Token {
            kind: TokenKind::Atom(atom),
            //span: panic!(), // TODO
        })
    }

    fn number(&mut self, start: usize) {
        let src = self.read_while(start, |c| c.is_digit(10));
        let value = isize::from_str(src).expect("Couldn't parse number");
        let token = Token {
            kind: TokenKind::Atom(Atom::Number(value)),
            //span: panic!(), // TODO
        };
        self.result.push(token);
    }

    fn name(&mut self, start: usize) {
        let value = self.read_while(start, |c| c.is_alphabetic() || c.is_numeric());
        let token = Token {
            kind: TokenKind::Atom(Atom::Name(value)),
            //span: panic!(), // TODO
        };
        self.result.push(token);
    }

    fn comment(&mut self, start: usize) {
        self.read_while(start, |c| c != '\n');
    }

    fn read_while<F>(&mut self, start: usize, f: F) -> &'a str
    where
        F: Fn(char) -> bool,
    {
        while let Some(&(_, c)) = self.iter.peek() {
            if !f(c) {
                break;
            }
            self.iter.next();
        }

        let end = match self.iter.peek() {
            Some(&(i, _)) => i,
            None => self.input.len(),
        };

        &self.input[start..end]
    }

    fn open_bracket(&mut self, br: Bracket) {
        let mut new_result = Vec::new();
        mem::swap(&mut self.result, &mut new_result);
        self.tree_stack.push((br, new_result));
    }

    fn close_bracket(&mut self, closer: char) -> QlResult<()> {
        let (br, mut prev_result) = match self.tree_stack.pop() {
            Some(x) => x,
            None => {
                return lex_err!(Unexpected, closer);
            }
        };

        if br.close() != closer {
            return lex_err!(Unmatched, br.close(), closer);
        }

        mem::swap(&mut prev_result, &mut self.result);
        let token = Token {
            kind: TokenKind::Tree(br, prev_result),
            //span: panic!(), // TODO
        };

        self.result.push(token);

        Ok(())
    }

    // Pre-condition: self.string.is_none()
    fn open_string(&mut self, index: usize) {
        self.string = Some(index + 1);
    }

    // Pre-condition: self.string.is_some()
    fn close_string(&mut self, index: usize) {
        let start = self.string.take().expect("Missing string buffer in lexer");
        let token = Token {
            kind: TokenKind::Atom(Atom::String(&self.input[start..index])),
            //span: panic!(), // TODO
        };
        self.result.push(token);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_empty() {
        let lexer = Lexer::new("");
        let result = lexer.tokenise().unwrap();
        assert!(result.is_empty());

        let lexer = Lexer::new("   ");
        let result = lexer.tokenise().unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_numbers() {
        let lexer = Lexer::new("0 42 -3 56785665657656");
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(assert_number(&result[0]), 0);
        assert_eq!(assert_number(&result[1]), 42);
        assert_eq!(assert_number(&result[2]), -3);
        assert_eq!(assert_number(&result[3]), 56785665657656);
    }

    #[test]
    fn test_names() {
        let lexer = Lexer::new("a foo bar42 ላዊዲሞክ");
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(assert_name(&result[0]), "a");
        assert_eq!(assert_name(&result[1]), "foo");
        assert_eq!(assert_name(&result[2]), "bar42");
        assert_eq!(assert_name(&result[3]), "ላዊዲሞክ");

        let lexer = Lexer::new("a\nb");
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(assert_name(&result[0]), "a");
        assert_eq!(assert_atom(&result[1]), Atom::NewLine);
        assert_eq!(assert_name(&result[2]), "b");
    }

    #[test]
    fn test_atoms() {
        let lexer = Lexer::new(":,! !");
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 4);
        assert_eq!(assert_atom(&result[0]), Atom::Colon);
        assert_eq!(assert_atom(&result[1]), Atom::Comma);
        assert_eq!(assert_atom(&result[2]), Atom::Bang);
        assert_eq!(assert_atom(&result[3]), Atom::Bang);
    }

    #[test]
    // Also tests newline/whitespace handling.
    fn test_strings() {
        let lexer = Lexer::new("  \"foo\"\r\n    \"bar\"");
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(assert_string(&result[0]), "foo");
        assert_eq!(assert_atom(&result[1]), Atom::NewLine);
        assert_eq!(assert_string(&result[2]), "bar");
    }

    #[test]
    fn test_query() {
        let input = r"{
          human(id: 1002) {
            name,
            appearsIn,
            id
          }
        }";
        let lexer = Lexer::new(input);
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].to_string().replace(' ', ""),
            input.replace(' ', "")
        );
    }

    #[test]
    fn test_query_with_comments() {
        let input = r"{
          # A comment
          human(id: 1002) {
            name # field comment
            appearsIn
            id
          }
        }";

        let expected = r"{

          human(id: 1002) {
            name
            appearsIn
            id
          }
        }";
        let lexer = Lexer::new(input);
        let result = lexer.tokenise().unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].to_string().replace(' ', ""),
            expected.replace(' ', "")
        );
    }

    // TODO test: errors

    fn assert_number(tok: &Token) -> isize {
        match assert_atom(tok) {
            Atom::Number(n) => n,
            _ => panic!("Non-number token, expected number"),
        }
    }

    fn assert_name<'a>(tok: &Token<'a>) -> &'a str {
        match assert_atom(tok) {
            Atom::Name(n) => n,
            _ => panic!("Non-number token, expected name"),
        }
    }

    fn assert_string<'a>(tok: &Token<'a>) -> &'a str {
        match assert_atom(tok) {
            Atom::String(s) => s,
            _ => panic!("Non-string token, expected string"),
        }
    }

    fn assert_atom<'a>(tok: &Token<'a>) -> Atom<'a> {
        match tok.kind {
            TokenKind::Atom(atom) => atom,
            _ => panic!("Non-atomic token"),
        }
    }
}
