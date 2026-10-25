use std::collections::HashMap;

use regex::Regex;

use crate::renamer::{Path, PathSegment};
use crate::renamer::token::{Parser, Token};

pub enum InfixOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Contains,
    Matches,
    // Is,
    And,
    Or,
    Xor,
}

impl InfixOp {
    fn from_token(tok: Token) -> Self {
        match tok {
            Token::Equal => Self::Equal,
            Token::NotEqual => Self::NotEqual,
            Token::GreaterThan => Self::GreaterThan,
            Token::LessThan => Self::LessThan,
            Token::GreaterEqual => Self::GreaterEqual,
            Token::LessEqual => Self::LessEqual,
            Token::Contains => Self::Contains,
            Token::Matches => Self::Matches,
            // Token::KywdIs => Self::Is,
            Token::And => Self::And,
            Token::Or => Self::Or,
            Token::Xor => Self::Xor,
            t => panic!("unexpected infix operator token: {t:?}")
        }
    }
}

pub enum PrefixOp {
    Not,
}

// @todo: support array expressions
// @todo: add literals (strings, regexes, integers, floating points) -> only these are allowed in arrays
pub enum Expr<'src> {
    String(&'src str),
    Regex(Regex),
    Path(Path),
    Func(FuncCall<'src>),
    // should i distinguish infix operators (`==`, `<`) from logical operators (`&&`, `||`)?
    InfixOp(Box<Self>, InfixOp, Box<Self>),
    PrefixOp(PrefixOp, Box<Self>),
    Test(Test<'src>),
}

impl<'src> Expr<'src> {
    pub fn parse(parser: &Parser<'src>, min_bind_pow: u8) -> Self {
        let mut lhs = match parser.peek() {
            (Token::String(s), _) => {
                let _ = parser.next();
                Self::String(s)
            }
            (Token::Regex(r), _) => {
                let _ = parser.next();
                Self::Regex(Regex::new(r).unwrap())
            }
            (Token::Ident(name), _) => {
                let _ = parser.next();
                if let (Token::OpenParen, _) = parser.peek() {
                    Self::Func(FuncCall::parse(parser, name))
                } else {
                    let first_seg = PathSegment::Name(name.into());
                    Self::Path(Path::parse(parser, first_seg))
                }
            }
            (Token::Dot, _) => {
                let _ = parser.next();
                let first_seg = PathSegment::parse(parser);
                Self::Path(Path::parse(parser, first_seg))
            }
            (Token::Not, _) => {
                let _ = parser.next();
                Self::PrefixOp(
                    PrefixOp::Not,
                    Box::new(Self::parse(parser, prefix_assoc(&Token::Not) + 1))
                )
            }
            (Token::OpenBrace, _) => {
                parser.consume(&Token::OpenBrace);
                let expr = Expr::parse(parser, 0);
                parser.consume(&Token::CloseBrace);
                expr
            }
            (t, s) => panic!("unexpected expression start token: {t:?} @ {s:?}"),
        };

        loop {
            // @todo: categorize the binary operators
            // ^^ string matching operators may require different handling than equality operators
            let op = match parser.peek() {
                (op @ Token::Equal
                    | op @ Token::NotEqual
                    | op @ Token::GreaterThan
                    | op @ Token::LessThan
                    | op @ Token::GreaterEqual
                    | op @ Token::LessEqual
                    | op @ Token::Contains
                    | op @ Token::Matches
                    | op @ Token::KywdIs
                    | op @ Token::KywdIn
                    | op @ Token::And
                    | op @ Token::Or
                    | op @ Token::Xor, _) => op,
                (Token::Eof | Token::CloseBrace | Token::CloseParen | Token::Comma, _) => break,
                (t, s) => panic!("unexpected operator token: {t:?} @ {s:?}"),
            };
            let infix_bind_pow = infix_assoc(&op);

            if infix_bind_pow.0 >= min_bind_pow {
                parser.consume(&op);
                lhs = match op {
                    Token::KywdIs => {
                        match parser.next().0 {
                            Token::Ident(ident) => {
                                Self::Test(Test::parse(parser, ident, Box::new(lhs)))
                            }
                            t => panic!("unexpected token: {t:?}")
                        }
                    }
                    _ => {
                        let rhs = Self::parse(parser, infix_bind_pow.1);
                        Self::InfixOp(Box::new(lhs), InfixOp::from_token(op), Box::new(rhs))
                    }
                };
                continue;
            }
            break;
        }
        lhs
    }
}

pub struct FuncCall<'src> {
    pub name: smol_str::SmolStr,
    pub args: Option<Args<'src>>
}

impl<'src> FuncCall<'src> {
    fn parse(parser: &Parser<'src>, name: &str) -> Self {
        parser.consume(&Token::OpenParen);
        let args = if parser.at(&Token::CloseParen) {
            None
        } else {
            Some(Args::parse(parser))
        };
        parser.consume(&Token::CloseParen);
        FuncCall { name: name.into(), args }
    }
}

pub struct Test<'src> {
    pub name: smol_str::SmolStr,
    pub args: Option<Args<'src>>,
    pub receiver: Box<Expr<'src>>,
}

impl<'src> Test<'src> {
    fn parse(parser: &Parser<'src>, name: &str, receiver: Box<Expr<'src>>) -> Self {
        let args = if parser.at(&Token::OpenParen) {
            parser.consume(&Token::OpenParen);
            let args = Args::parse(parser);
            parser.consume(&Token::CloseParen);
            Some(args)
        } else { None };
        Self {
            name: name.into(),
            args,
            receiver,
        }
    }
}

pub struct Args<'src>(pub HashMap<smol_str::SmolStr, Expr<'src>>);

impl<'src> Args<'src> {
    fn parse(parser: &Parser<'src>) -> Self {
        let mut args = HashMap::new();
        loop {
            // @todo: add a check for ident here
            if let (Token::Ident(id), _) = parser.peek() {
                parser.consume(&Token::Ident(id));
                parser.consume(&Token::Assign);
                let expr = Expr::parse(parser, 0);
                args.insert(id.into(), expr);
            }
            if !parser.at(&Token::Comma) {
                break;
            }
            parser.consume(&Token::Comma);
        }
        Self(args)
    }
}

fn infix_assoc(op: &Token) -> (u8, u8) {
    match op {
        Token::Or => (1, 2), // left associative
        Token::And | Token::Xor => (3, 4), // left associative
        Token::Equal | Token::NotEqual => (5, 6), // left associative
        Token::GreaterThan | Token::LessThan
            | Token::GreaterEqual | Token::LessEqual => (7, 8),
        // i need to double check this associativity value (does it bind more or less than `>`)
        Token::Contains | Token::Matches | Token::KywdIs | Token::KywdIn => (9, 10), // left associative
        t => panic!("unsupported infix operator: {t:?}")
    }
}
fn prefix_assoc(op: &Token) -> u8 {
    match op {
        Token::Not => 50,
        t => panic!("unsupported prefix operator: {t:?}")
    }
}
