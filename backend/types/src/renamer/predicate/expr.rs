// vvv ripped form `upon`

use std::collections::HashMap;

use crate::renamer::{Path, PathSegment};
use crate::renamer::token::{Parser, Token};

#[derive(Debug)]
pub enum Expr {
    Simple(SimpleExpr),
    Filter(Filter),
}

impl<'src> Expr {
    pub fn parse(parser: &Parser<'src>) -> Self {
        let mut expr = Expr::Simple(SimpleExpr::parse(parser));
        while parser.at(&Token::Pipe) {
            parser.consume(&Token::Pipe);
            let tok = parser.next().0;
            match tok {
                Token::Ident(id) => {
                     expr = Expr::Filter(Filter::parse(parser, id, Box::new(expr)))
                }
                t => panic!("unexpected token: {t:?}"),
            }
        }
        expr
    }
}

#[derive(Debug)]
pub enum SimpleExpr {
    // Unquoted(&'src str),
    Path(Path),
    String(smol_str::SmolStr),
    // FormatStr(&'src str),
    Block(Box<Expr>), // a nested expression set
    Func(FuncCall),
}

impl<'src> SimpleExpr {
    fn parse(parser: &Parser<'src>) -> Self {
        let tok = parser.next().0;
        match tok {
            Token::String(s) => SimpleExpr::String(smol_str::SmolStr::new(s)),
            Token::Ident(id) => {
                if let (Token::OpenParen, _) = parser.peek() {
                    // this is a function call
                    SimpleExpr::Func(FuncCall::parse(parser, id))
                } else {
                    // path segment
                    let first_seg = PathSegment::Name(id.into());
                    SimpleExpr::Path(Path::parse(parser, first_seg))
                }
            }
            Token::Dot => {
                let first_seg = PathSegment::parse(parser);
                SimpleExpr::Path(Path::parse(parser, first_seg))
            }
            Token::OpenBrace => {
                let expr = SimpleExpr::Block(Box::new(Expr::parse(parser)));
                parser.consume(&Token::CloseBrace);
                expr
            }
            tok => panic!("unexpected token: {tok:?}"),
        }
    }
}

#[derive(Debug)]
pub struct Filter {
    pub name: smol_str::SmolStr,
    pub args: Option<Args>,
    pub receiver: Box<Expr>,
}

impl<'src> Filter {
    fn parse(parser: &Parser<'src>, name: &str, receiver: Box<Expr>) -> Self {
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

#[derive(Debug)]
pub struct Args(pub HashMap<smol_str::SmolStr, SimpleExpr>);

impl<'src> Args {
    fn parse(parser: &Parser<'src>) -> Self {
        let mut args = HashMap::new();
        loop {
            // @todo: add a check for ident here
            if let (Token::Ident(id), _) = parser.peek() {
                parser.consume(&Token::Ident(id));
                parser.consume(&Token::Assign);
                let expr = SimpleExpr::parse(parser);
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

#[derive(Debug)]
pub struct FuncCall {
    pub name: smol_str::SmolStr,
    pub args: Option<Args>
}

impl<'src> FuncCall {
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
