#[derive(Clone, Debug)]
pub struct Token<'a> {
    pub kind: TokenKind<'a>,
    // FIXME cfg this
    // TODO spans
    // pub span: ::lexer::Span,
}

#[derive(Clone, Debug)]
pub enum TokenKind<'a> {
    Atom(Atom<'a>),
    Tree(Bracket, Vec<Token<'a>>),
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum Bracket {
    Brace,
    Paren,
    Square,
}

impl Bracket {
    pub fn open(&self) -> char {
        match *self {
            Bracket::Brace => '{',
            Bracket::Paren => '(',
            Bracket::Square => '[',
        }
    }

    pub fn close(&self) -> char {
        match *self {
            Bracket::Brace => '}',
            Bracket::Paren => ')',
            Bracket::Square => ']',
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Copy)]
pub enum Atom<'a> {
    NewLine,
    Comma,
    Colon,
    Bang,
    Name(&'a str),
    Number(isize),
    String(&'a str),
}

mod display {
    use super::*;
    use std::fmt::{Display, Formatter, Result};

    impl<'a> Display for Token<'a> {
        fn fmt(&self, f: &mut Formatter) -> Result {
            self.kind.fmt(f)
        }
    }

    impl<'a> Display for TokenKind<'a> {
        fn fmt(&self, f: &mut Formatter) -> Result {
            match *self {
                TokenKind::Atom(ref a) => a.fmt(f),
                TokenKind::Tree(ref b, ref ts) => {
                    write!(f, "{}", b.open())?;
                    write!(
                        f,
                        "{}",
                        ts.iter()
                            .map(|t| t.to_string())
                            .collect::<Vec<_>>()
                            .join(" ")
                    )?;
                    write!(f, "{}", b.close())
                }
            }
        }
    }

    impl<'a> Display for Atom<'a> {
        fn fmt(&self, f: &mut Formatter) -> Result {
            match *self {
                Atom::NewLine => writeln!(f, ""),
                Atom::Comma => write!(f, ","),
                Atom::Colon => write!(f, ":"),
                Atom::Bang => write!(f, "!"),
                Atom::Name(n) => write!(f, "{}", n),
                Atom::Number(n) => write!(f, "{}", n),
                Atom::String(s) => write!(f, "\"{}\"", s),
            }
        }
    }
}
