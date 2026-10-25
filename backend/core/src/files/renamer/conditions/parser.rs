use logos::{Logos, Span, SpannedIter};

use crate::files::renamer::{Parser, Path, PathSegment};

#[derive(Debug, Clone, PartialEq, Default, Logos)]
#[logos(skip r"\s+")]
pub enum Token<'src> {
    // @todo: add grouped conditions (in parenthesis)
    // @todo: add function calls
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

pub enum InfixOp {
    Equal,
    NotEqual,
    GreaterThan,
    LessThan,
    GreaterEqual,
    LessEqual,
    Contains,
    Matches,
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

pub enum Expr<'src> {
    String(&'src str),
    Path(Path<'src>),
    // should i distinguish infix operators (`==`, `<`) from logical operators (`&&`, `||`)?
    InfixOp(Box<Self>, InfixOp, Box<Self>),
    PrefixOp(PrefixOp, Box<Self>),
}

impl<'src> Expr<'src> {
    pub fn new(parser: &mut Parser<'src, Token<'src>>, min_bind_pow: u8) -> Self {
        let mut lhs = match parser.peek().0 {
            Token::String(s) => {
                let _ = parser.next();
                Self::String(s)
            }
            Token::Ident(name) => {
                let _ = parser.next();
                let first_seg = PathSegment::Name(name);
                Self::Path(Path::new_from_condition(parser, first_seg))
            }
            Token::Dot => {
                let _ = parser.next();
                let first_seg = PathSegment::new_from_condition(parser);
                Self::Path(Path::new_from_condition(parser, first_seg))
            }
            Token::Not => {
                let _ = parser.next();
                Self::PrefixOp(
                    PrefixOp::Not,
                    Box::new(Self::new(parser, prefix_assoc(&Token::Not) + 1))
                )
            }
            t => panic!("unexpected expression start token: {t:?}"),
        };

        loop {
            // @todo: categorize the binary operators
            // ^^ string matching operators may require different handling than equality operators
            let op = match parser.peek().0 {
                op @ Token::Equal
                    | op @ Token::NotEqual
                    | op @ Token::GreaterThan
                    | op @ Token::LessThan
                    | op @ Token::GreaterEqual
                    | op @ Token::LessEqual
                    | op @ Token::Contains
                    | op @ Token::Matches
                    | op @ Token::And
                    | op @ Token::Or
                    | op @ Token::Xor => op,
                Token::Eof => break,
                t => panic!("unexpected operator token: {t:?}"),
            };
            let infix_bind_pow = infix_assoc(&op);

            if infix_bind_pow.0 >= min_bind_pow {
                parser.consume(&op);
                let rhs = Self::new(parser, infix_bind_pow.1);
                lhs = Self::InfixOp(Box::new(lhs), InfixOp::from_token(op), Box::new(rhs));
                continue;
            }
            break;
        }
        lhs
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
        Token::Contains | Token::Matches => (9, 10), // left associative
        t => panic!("unsupported infix operator: {t:?}")
    }
}
fn prefix_assoc(op: &Token) -> u8 {
    match op {
        Token::Not => 50,
        t => panic!("unsupported prefix operator: {t:?}")
    }
}
