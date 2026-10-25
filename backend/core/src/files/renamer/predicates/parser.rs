use logos::{Logos, Span, SpannedIter};

use crate::files::renamer::{Parser, Path, PathSegment};

#[derive(Debug, Clone, Logos, PartialEq, Default)]
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
    #[token(".")]
    Dot, // separates field accesses
    #[token("|")]
    Pipe, // separates expressions
    #[token(",")]
    Comma,

    #[regex(r#""((\\"|\\\\)|[^\\"])*""#, |lex| lex.slice().trim_matches('"'))]
    String(&'src str),
    // #[regex(r#"f"((\\"|\\\\)|[^\\"])*""#, |lex| lex.slice().trim_matches('"'))]
    // FormatString(&'src str),
    #[regex(r"[[:alpha:]]\w*")]
    Ident(&'src str),
    // @todo: assert that this token always follows a `Dot` (disallow freestanding numbers)
    // #[regex(r"\d+", priority = 3, callback = |lex| lex.slice().parse::<usize>().unwrap())]
    // Index(usize),
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

// vvv ripped form `upon`

#[derive(Debug)]
pub enum Expr<'src> {
    Simple(SimpleExpr<'src>),
    Filter(Filter<'src>),
}

impl<'src> Expr<'src> {
    pub fn new(parser: &mut Parser<'src, Token<'src>>) -> Self {
        let mut expr = Expr::Simple(SimpleExpr::new(parser));
        while parser.at(&Token::Pipe) {
            parser.consume(&Token::Pipe);
            let tok = parser.next().0;
            match tok {
                Token::Ident(id) => {
                     expr = Expr::Filter(Filter::new(parser, id, Box::new(expr)))
                }
                t => panic!("unexpected token: {t:?}"),
            }
        }
        expr
    }
}

#[derive(Debug)]
pub enum SimpleExpr<'src> {
    // Unquoted(&'src str),
    Path(Path<'src>),
    String(&'src str),
    // FormatStr(&'src str),
    Block(Box<Expr<'src>>), // a nested expression set
    Func(FuncCall<'src>),
}

impl<'src> SimpleExpr<'src> {
    fn new(parser: &mut Parser<'src, Token<'src>>) -> Self {
        let tok = parser.next().0;
        match tok {
            Token::String(s) => SimpleExpr::String(s),
            Token::Ident(id) => {
                if let (Token::OpenParen, _) = parser.peek() {
                    // this is a function call
                    SimpleExpr::Func(FuncCall::new(parser, id))
                } else {
                    // path segment
                    let first_seg = PathSegment::Name(id);
                    SimpleExpr::Path(Path::new_from_predicate(parser, first_seg))
                }
            }
            Token::Dot => {
                let first_seg = PathSegment::new_from_predicate(parser);
                SimpleExpr::Path(Path::new_from_predicate(parser, first_seg))
            }
            Token::OpenBrace => {
                let expr = SimpleExpr::Block(Box::new(Expr::new(parser)));
                parser.consume(&Token::CloseBrace);
                expr
            }
            tok => panic!("unexpected token: {tok:?}"),
        }
    }
}

#[derive(Debug)]
pub struct Filter<'src> {
    pub name: &'src str,
    pub args: Option<Args<'src>>,
    pub receiver: Box<Expr<'src>>,
}

impl<'src> Filter<'src> {
    fn new(parser: &mut Parser<'src, Token<'src>>, name: &'src str, receiver: Box<Expr<'src>>) -> Self {
        let args = if parser.at(&Token::OpenParen) {
            parser.consume(&Token::OpenParen);
            let args = Args::new(parser);
            parser.consume(&Token::CloseParen);
            Some(args)
        } else { None };
        Self {
            name,
            args,
            receiver,
        }
    }

}

#[derive(Debug)]
pub struct Args<'src>(pub Vec<SimpleExpr<'src>>);

impl<'src> Args<'src> {
    fn new(parser: &mut Parser<'src, Token<'src>>) -> Self {
        let mut args = Vec::new();
        loop {
            args.push(SimpleExpr::new(parser));
            if !parser.at(&Token::Comma) {
                break;
            }
            parser.consume(&Token::Comma);
        }
        Self(args)
    }
}

#[derive(Debug)]
pub struct FuncCall<'src> {
    pub name: &'src str,
    pub args: Option<Args<'src>>
}

impl<'src> FuncCall<'src> {
    fn new(parser: &mut Parser<'src, Token<'src>>, name: &'src str) -> Self {
        parser.consume(&Token::OpenParen);
        let args = if parser.at(&Token::CloseParen) {
            None
        } else {
            Some(Args::new(parser))
        };
        parser.consume(&Token::CloseParen);
        FuncCall { name, args }
    }
}
