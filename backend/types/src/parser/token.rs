use std::range::Range;

use itertools::PeekNth;
use logos::{Logos, SpannedIter};
use uncased::UncasedStr;
use unicode_general_category::{GeneralCategory, get_general_category};

use super::keyword::{KEYWORDS, Keyword};

// \p{Pd} -- any dash character (e.g. -)
// \p{Zs} -- any space character
// \p{Ps} -- any open punctuation (e.g., (, [, {)
// \p{Pe} -- any close punctuation (e.g. ), ], })
// \p{Po} -- any other punctuation
// \p{Pc} -- any connector punctuation
// \p{Pi} -- any initial punctuation (e.g. “, «)
// \p{Pf} -- any final punctuation (e.g. ”, »)
// \p{Sm} -- any math symbol (e.g. +, -, ~)
// \pS    -- any symbol
// \pL    -- any letter
// \pN    -- any number
// #[derive(Debug, Logos, Clone)]
#[derive(Debug, Clone)]
pub enum TokenKind<'t> {
    // #[regex(r"\p{Ps}+")]
    OpenPunc(&'t str),
    // #[regex(r"\p{Pe}+")]
    ClosePunc(&'t str),
    // any codepoint not classified as a delimeter, brace/bracket, or whitespace
    // #[regex(r"(?:[\pL\pN]|[\p{Po}&&[^.,&]]|[\pS&&[^+|]])+")]
    Text(&'t str),
    // number parsing comes later, higher priority to rip classification away from (and prevent clashing with) `Text`
    // #[regex(r"\pN+", priority = 3)]
    Numeric(&'t str),
    // this is needed to eventually combine tokens
    // keep the spaces separate for later
    // #[regex(r"(?:[\p{Pi}\p{Pf}\p{Pd}\p{Pc}\.,&+\|]+|\p{Zs}+)")]
    Delimiter(&'t str),
    Invalid,
    Eof,
}

impl TokenKind<'_> {
    fn len(&self) -> usize {
        match self {
            Self::Eof | Self::Invalid => 0,
            Self::OpenPunc(t) | Self::ClosePunc(t) | Self::Delimiter(t)
                | Self::Numeric(t) | Self::Text(t) => t.len()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Token<'t> {
    pub kind: TokenKind<'t>,
    pub keyword: Option<Keyword>,
    pub span: std::range::Range<usize>,
    // mainly for upper-level item parsing
    // have we seen this token yet?
    pub is_known: bool,
    // this is token inside a set of `OpenPunc` and `ClosePunc`?
    pub is_enclosed: bool,
}

impl Token<'_> {
    pub fn is_free(&self) -> bool {
        matches!(self.kind, TokenKind::Text(_) | TokenKind::Numeric(_)) && !self.is_known
    }
    pub fn is_bracket(&self) -> bool {
        matches!(self.kind, TokenKind::OpenPunc(_) | TokenKind::ClosePunc(_))
    }
    pub fn is_numeric(&self) -> bool {
        matches!(self.kind, TokenKind::Numeric(_))
    }
}

// @todo: rework the lexing process to remove the `logos` dependency (handroll token processing)
// ^^ `logos` is causing more problems than it fixes here
// pub struct TokenIter<'t> {
//     src: &'t str,
//     tokens: PeekNth<SpannedIter<'t, TokenKind<'t>>>,
//     scope_level: u8,
// }
//
// impl<'t> TokenIter<'t> {
//     pub fn new(src: &'t str) -> Self {
//         Self {
//             src,
//             tokens: itertools::peek_nth(TokenKind::lexer(src).spanned()),
//             scope_level: 0,
//         }
//     }
// }

// impl<'t> Iterator for TokenIter<'t> {
//     type Item = Token<'t>;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         let enclosed = self.scope_level > 0;
//         match self.tokens.next()? {
//             (Ok(w @ (TokenKind::Text(t) | TokenKind::Numeric(t))), span) => {
//                 let mut token = if matches!(w, TokenKind::Text(_)) {
//                     // if the text token is a keyword, break the function immediately
//                     // as no further processing is required
//                     // should there be a designated token for keywords?
//                     //
//                     // @fixme: this can break if the keyword is potentially more than 1 token long (e.g. `WEB-DL`)
//                     // as this chooses the single word equivalent (`WEB`)
//                     // if let Some(kywd) = KEYWORDS.get(t.into()) {
//                     //     return Some(Token {
//                     //         kind: TokenKind::Text(t),
//                     //         keyword: Some(kywd.clone()),
//                     //         span: Range::from(span.clone()),
//                     //         is_known: false,
//                     //         is_enclosed: enclosed,
//                     //     })
//                     // }
//                     Some(Token {
//                         kind: TokenKind::Text(t),
//                         keyword: None,
//                         span: Range::from(span.clone()),
//                         is_known: false,
//                         is_enclosed: enclosed,
//                     })
//                 } else {
//                     Some(Token {
//                         kind: TokenKind::Numeric(t),
//                         keyword: None,
//                         span: Range::from(span.clone()),
//                         is_known: false,
//                         is_enclosed: enclosed,
//                     })
//                 };
//                 // tokens is cloned here since i need access to multiple elements at the same time
//                 let mut ending_tok_tokens = self.tokens.clone();
//
//                 // taken from Rapptz/anitomy-rs
//                 if let Some((Ok(TokenKind::Delimiter(d)), _)) = self.tokens.peek() {
//                     let ending_tok = ending_tok_tokens.peek_nth(1)
//                         .unwrap_or(&(Ok(TokenKind::Eof), 0..0));
//
//                     // check if the combined token would produce a keyword (e.g. `mpeg-2`, `vc-1`, `Dolby Vision`)
//                     if !matches!(ending_tok.0, Ok(TokenKind::Eof) | Ok(TokenKind::Invalid) | Err(_))
//                         && let Some(kywd) = KEYWORDS.get(self.src[span.start..ending_tok.1.end].into())
//                     {
//                         let kind = TokenKind::Text(&self.src[span.start..ending_tok.1.end]);
//                         let span = Range::from(span.start..ending_tok.1.end);
//
//                         self.tokens.next();
//                         self.tokens.next();
//                         token = Some(Token {
//                             kind,
//                             keyword: Some(kywd.clone()),
//                             span,
//                             is_known: false,
//                             is_enclosed: enclosed,
//                         })
//                     } else if let Some(kywd) = KEYWORDS.get((&self.src[span.start..span.end]).into()) {
//                         // process single word keywords *after* we've checked for multi-word keywords
//                         // since some keywords (e.g. WEB-DL) also match single word keywords (e.g. WEB)
//                         token = Some(Token {
//                             kind: TokenKind::Text(&self.src[span.start..span.end]),
//                             keyword: Some(kywd.clone()),
//                             span: span.into(),
//                             is_known: false,
//                             is_enclosed: enclosed,
//                         });
//                     } else if d.starts_with(['-', '.', '&', '+', '~']) {
//                         // if the ending token is a keyword (e.g. `1080p`), break out here to avoid
//                         // combining into an invalid token (e.g. `2020.1080p`)
//                         if let Ok(TokenKind::Text(txt)) = ending_tok.0
//                             && KEYWORDS.get(txt.into()).is_some() { return token; }
//                         if let (TokenKind::Numeric(b), Ok(TokenKind::Numeric(a) | TokenKind::Text(a))) = (w.clone(), ending_tok.0.clone())
//                         {
//                             // leave the tokens be if they'll form a year range (i.e. `1982-1992`)
//                             if is_year(b.parse().unwrap_or(i16::MAX)) && is_year(a.parse().unwrap_or(i16::MAX))
//                                 && *d == "-"
//                             // fall through this if block into the code below
//                             {}
//                             // if at least one token is a year, break out here (e.g. `1900.1998`, `2.2017`, `2005.8x11`)
//                             else if is_year(b.parse().unwrap_or(i16::MAX)) || is_year(a.parse().unwrap_or(i16::MAX))
//                             {
//                                 return token;
//                             }
//                         }
//
//                         if (is_mostly_numbers(w.clone()) && is_mostly_numbers(ending_tok.0.as_ref().unwrap_or(&TokenKind::Invalid).clone()))
//                             || (*d == "."
//                                 && matches!(w, TokenKind::Text(_))
//                                 && t == UncasedStr::new("No")
//                                 && matches!(ending_tok.0, Ok(TokenKind::Numeric(_)))
//                             )
//                             // combine tokens of the format `S01E01` and `E02` or `02`
//                             || (*d == "-"
//                                 && t.starts_with('S')
//                                 && t.contains('E')
//                                 && matches!(ending_tok.0.clone(), Ok(t) if is_mostly_numbers(t.clone()))
//                             )
//                         {
//                                 let kind = TokenKind::Text(&self.src[span.start..ending_tok.1.end]);
//                                 let span = Range::from(span.start..ending_tok.1.end);
//                                 // advance iterator to ending token
//                                 self.tokens.next();
//                                 self.tokens.next();
//                                 token = Some(Token {
//                                     kind,
//                                     keyword: None,
//                                     span,
//                                     is_known: false,
//                                     is_enclosed: enclosed,
//                                 });
//                             }
//                     }
//                 }
//                 token
//             }
//             (Err(_), span) => {
//                 Some(Token {
//                     kind: TokenKind::Invalid,
//                     keyword: None,
//                     span: Range::from(span.clone()),
//                     is_known: false,
//                     is_enclosed: enclosed,
//                 })
//             }
//             (Ok(TokenKind::OpenPunc(p)), span) => {
//                 self.scope_level += 1;
//                 Some(Token {
//                     kind: TokenKind::OpenPunc(p),
//                     keyword: None,
//                     span: Range::from(span.clone()),
//                     is_known: false,
//                     is_enclosed: self.scope_level > 1,
//                 })
//             }
//             (Ok(TokenKind::ClosePunc(p)), span) => {
//                 self.scope_level -= 1;
//                 Some(Token {
//                     kind: TokenKind::ClosePunc(p),
//                     keyword: None,
//                     span: Range::from(span.clone()),
//                     is_known: false,
//                     is_enclosed: self.scope_level > 1,
//                 })
//             }
//             (Ok(t), span) => {
//                 Some(Token {
//                     kind: t,
//                     keyword: None,
//                     span: Range::from(span),
//                     is_known: false,
//                     is_enclosed: enclosed,
//                 })
//             }
//         }
//     }
// }

// from anitomy-rs
pub struct Lexer<'src> {
    inp: &'src str,
}

impl<'src> Lexer<'src> {
    pub fn new(inp: &'src str) -> Self {
        Self { inp }
    }

    fn is_empty(&self) -> bool { self.inp.is_empty() }

    fn take_keyword(&mut self) -> Option<(&'src str, Keyword)> {
        let mut key = "";

        // march through current input and find the first matching keyword
        for (idx, ch) in self.inp.char_indices() {
            // include *all* input before and including this character
            let prefix = &self.inp[0..(idx + ch.len_utf8())];

            if KEYWORDS.contains_key(prefix.into()) {
                key = prefix;
            }
            // do any keys start with the current prefix?
            if KEYWORDS.keys().filter(|k| k.starts_with(prefix)).count() > 0 {
                continue;
            }
            // no keyword matches the prefix
            if key.is_empty() { return None; }
        }

        let kywd = KEYWORDS.get(key.into())?;
        let rest = &self.inp[key.len()..];
        if kywd.is_bounded() && !is_keyword_boundary(rest) {
            if !(kywd.is_ambiguous() && rest.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(true)) {
                return None;
            }
        }

        // shift the input forward to after the keyword
        let (b, a) = self.inp.split_at(key.len());
        if b.is_empty() {
            None
        } else {
            self.inp = a;
            Some((b, kywd.clone()))
        }
    }

    fn take_text(&mut self) -> &'src str {
        if let Some((idx, ch)) = self.inp.char_indices()
            .take_while(|(_,c)| is_text(*c))
            .last()
        {
            let new_idx = idx + ch.len_utf8();
            let (b, a) = self.inp.split_at(new_idx);
            self.inp = a;
            b
        } else { "" }
    }

    fn take_if<P: Fn(char) -> bool>(&mut self, pred: P) -> Option<&'src str> {
        let ch = self.inp.chars().next()?;
        if pred(ch) {
            let val = &self.inp[0..ch.len_utf8()];
            self.inp = &self.inp[ch.len_utf8()..];
            Some(val)
        } else { None }
    }

    pub fn into_tokens(self) -> Vec<Token<'src>> {
        let orig_text = self.inp;
        let mut tokens = self.into_iter().collect::<Vec<_>>();
        let mut start_len = 0;

        for idx in 0..tokens.len() {
            let tok_len = tokens[idx].kind.len();
            let is_dot = matches!(tokens[idx].kind, TokenKind::Delimiter("."));

            if let TokenKind::Delimiter(delim) = tokens[idx].kind
                && delim.starts_with(['.', '-', '&', '+', '~'])
            {
                if let Ok([prev, next]) = tokens.get_disjoint_mut([idx - 1, idx + 1]) {
                    if let (TokenKind::Numeric(prev), TokenKind::Numeric(next) | TokenKind::Text(next)) = (prev.kind.clone(), next.kind.clone()) {
                        // do not merge tokens when both are a year (e.g. `1982-1992`)
                        if is_year(prev.parse().unwrap_or(i16::MAX)) && is_year(next.parse().unwrap_or(i16::MAX))
                            && delim == "-"
                        {}
                        // do not merge when at least one token is a year (e.g. `1900.1998`, `2.2017`, `2005.8x11`)
                        // break out here
                        else if is_year(prev.parse().unwrap_or(i16::MAX)) || is_year(next.parse().unwrap_or(i16::MAX))
                        { break; }
                    }
                    if (is_mostly_numbers(prev.kind.clone()) && is_mostly_numbers(next.kind.clone()))
                        || (is_dot
                            && matches!(prev.kind, TokenKind::Text(t) if t == UncasedStr::new("No"))
                            && matches!(next.kind, TokenKind::Numeric(_))
                        )
                    {
                        // if !(matches!(prev.kind, TokenKind::Numeric(n) if is_year(n.parse().unwrap_or(i16::MAX)))
                        //     || matches!(next.kind, TokenKind::Numeric(n) if is_year(n.parse().unwrap_or(i16::MAX))))
                        // {
                            let start = start_len - prev.kind.len();
                            let end = start_len + tok_len + next.kind.len();

                            prev.kind = TokenKind::Invalid;
                            next.kind = TokenKind::Invalid;

                            // merge the tokens (delimeter in the center)
                            tokens[idx].kind = TokenKind::Text(&orig_text[start..end]);
                            tokens[idx].span = Range::from(start..end);
                        // }
                    }
                }
            }
            start_len += tok_len;
        }

        tokens.retain(|t| !matches!(t.kind, TokenKind::Invalid));
        start_len = 0;
        for tok in tokens.iter_mut() {
            let tok_span = tok.kind.len();
            tok.span = Range::from(start_len..start_len+tok_span);
            start_len += tok_span;
        }

        tokens
    }
}

pub struct TokenIter<'src> {
    lexer: Lexer<'src>,
    scope_level: usize,
}

impl<'src> TokenIter<'src> {
    fn new(lexer: Lexer<'src>) -> Self {
        Self {
            lexer,
            scope_level: 0,
        }
    }
}

impl<'src> IntoIterator for Lexer<'src> {
    type Item = Token<'src>;
    type IntoIter = TokenIter<'src>;

    fn into_iter(self) -> Self::IntoIter {
        TokenIter::new(self)
    }
}

impl<'src> Iterator for TokenIter<'src> {
    type Item = Token<'src>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.lexer.is_empty() { return None; }

        let is_enclosed = self.scope_level > 0;

        if let Some(val) = self.lexer
            .take_if(|c| matches!(get_general_category(c), GeneralCategory::OpenPunctuation))
        {
            self.scope_level += 1;
            return Some(Token {
                kind: TokenKind::OpenPunc(val),
                keyword: None,
                span: Range::from(0..0),
                is_known: false,
                is_enclosed: self.scope_level >= 2,
            })
        }
        if let Some(val) = self.lexer
            .take_if(|c| matches!(get_general_category(c), GeneralCategory::ClosePunctuation))
        {
            self.scope_level -= 1;
            return Some(Token {
                kind: TokenKind::ClosePunc(val),
                keyword: None,
                span: Range::from(0..0),
                is_known: false,
                is_enclosed: self.scope_level >= 1,
            })
        }
        if let Some(val) = self.lexer
            .take_if(is_delimeter)
        {
            return Some(Token {
                kind: TokenKind::Delimiter(val),
                keyword: None,
                span: Range::from(0..0),
                is_known: false,
                is_enclosed,
            })
        }

        match self.lexer.take_keyword() {
            Some((val, kywd)) => Some(Token {
                kind: TokenKind::Text(val),
                keyword: Some(kywd),
                span: Range::from(0..0),
                is_known: false,
                is_enclosed,
            }),
            None => {
                let text = self.lexer.take_text();
                if text.is_empty() {
                    None
                } else if text.as_bytes().iter().all(u8::is_ascii_digit) {
                    Some(Token {
                        kind: TokenKind::Numeric(text),
                        keyword: None,
                        span: Range::from(0..0),
                        is_known: false,
                        is_enclosed,
                    })
                } else {
                    Some(Token {
                        kind: TokenKind::Text(text),
                        keyword: None,
                        span: Range::from(0..0),
                        is_known: false,
                        is_enclosed,
                    })
                }
            }
        }
    }
}

fn is_bracket(ch: char) -> bool {
    matches!(get_general_category(ch),
        GeneralCategory::OpenPunctuation | GeneralCategory::ClosePunctuation)
}
fn is_delimeter(ch: char) -> bool {
    matches!(
        get_general_category(ch),
        GeneralCategory::InitialPunctuation | GeneralCategory::FinalPunctuation
        | GeneralCategory::DashPunctuation | GeneralCategory::ConnectorPunctuation
        | GeneralCategory::SpaceSeparator
    ) || ['.', ',', '&', '+', '|'].contains(&ch)
}
fn is_text(ch: char) -> bool {
    !is_bracket(ch) && !is_delimeter(ch)
}

fn is_keyword_boundary(s: &str) -> bool {
    s.chars().next().map(|c| !is_text(c)).unwrap_or(true)
}

// vvv taken from Rapptz/anitomy-rs
pub fn is_mostly_numbers(tok: TokenKind<'_>) -> bool {
    match tok {
        TokenKind::Text(w) => {
            if w.parse::<f64>().is_ok() { return true; }
            // w.len() is byte-length, not character length
            let codepts = w.chars().count();
            let nums = w.chars()
                .filter(|c| c.is_numeric() || cjk_numerals().contains(c))
                .count();
            // prevent counting tokens like `S01` as mostly numbers
            nums * 2 >= codepts && w.chars().next().is_some_and(|c| c != 'S' && c != 's')
        }
        // all classified as numbers
        TokenKind::Numeric(_) => true,
        _ => false,
    }
}
pub fn is_year(year: i16) -> bool {
    // reasonable year range (otherwise it begins to match valid episode numbers)
    (1900..2100).contains(&year)
}

pub fn cjk_numerals() -> [char; 12] {
    // @todo: check if korean numerals show up in practice (or any of these for that matter)
    ['〇', '零', '一', '二', '三', '四', '五', '六', '七', '八', '九', '十']
}

