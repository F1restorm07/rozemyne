use std::cell::RefCell;
use std::iter::Peekable;

use logos::{Logos, Span, SpannedIter};


#[derive(Debug, Clone, PartialEq, Default, Logos)]
#[logos(skip r"\s+")]
pub enum Token<'src> {
    #[token("(")]
    OpenParen, // indicates filter call
    #[token(")")]
    CloseParen,
    #[token("{")]
    OpenBrace, // indicates nested expression block
    #[token("}")]
    CloseBrace,
    #[token("|")]
    Pipe, // separates expressions
    #[token(",")]
    Comma, // separates filter/test arguments
    #[token("=")]
    Assign, // separates keywords from expressions

    #[token("==")]
    Equal,
    #[token("!=")]
    NotEqual,
    #[token(">")]
    GreaterThan,
    #[token("<")]
    LessThan,
    #[token(">=")]
    GreaterEqual,
    #[token("<=")]
    LessEqual,

    #[token("&&")]
    And,
    #[token("||")]
    Or,
    #[token("^^")]
    Xor,
    #[token("!")]
    Not,

    #[token(".")]
    Dot,

    #[token("contains")]
    Contains,
    #[token("matches")]
    Matches,
    #[token("in")]
    KywdIn, // is a value in a set?
    #[token("is")]
    KywdIs, // indicates a test follows

    // #[regex(r#"f"((\\"|\\\\)|[^\\"])*""#, |lex| lex.slice().trim_matches('"'))]
    // FormatString(&'src str),
    #[regex(r#"r"((\\"|\\\\)|[^\\"])*""#, |lex| lex.slice().trim_matches('"'))]
    Regex(&'src str),
    #[regex(r"[-+]?\d+", priority = 3, callback = |lex| lex.slice().parse::<i64>().unwrap())]
    Int(i64),

    #[regex(r#""((\\"|\\\\)|[^\\"])*""#, |lex| lex.slice().trim_matches('"'))]
    String(&'src str),
    #[regex(r"[[:alpha:]]\w*")]
    Ident(&'src str),
    #[default]
    Eof,
}

struct TokenIter<'src> {
    lexer: SpannedIter<'src, Token<'src>>,
}

impl<'src> TokenIter<'src> {
    fn new(src: &'src str) -> Self {
        Self { lexer: Token::lexer(src).spanned() }
    }
}

impl<'src> Iterator for TokenIter<'src> {
    type Item = (Token<'src>, Span);

    fn next(&mut self) -> Option<Self::Item> {
        self.lexer.next().map(|(t, s)| (t.unwrap_or(Token::Eof), s))
    }
}

pub struct Parser<'src> {
    tokens: RefCell<Peekable<TokenIter<'src>>>
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Self {
        Self { tokens: RefCell::new(TokenIter::new(src).peekable()) }
    }

    pub fn next(&self) -> (Token<'src>, Span) {
        self.tokens.borrow_mut().next().unwrap_or_default()
    }
    pub fn peek(&self) -> (Token<'src>, Span) {
        self.tokens.borrow_mut().peek().unwrap_or(&(Token::default(), 0..0)).clone()
    }
    pub fn at(&self, tok: &Token) -> bool {
        &self.peek().0 == tok
    }
    pub fn consume(&self, expected: &Token) {
        let (t, s) = self.next();
        if &t != expected {
            panic!("expected: {expected:?}, got: {t:?} at {s:?}")
        }
    }
}
