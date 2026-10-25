use backend_types::parser::token::{Token, TokenKind};
use unicode_general_category::{GeneralCategory, get_general_category};

pub use backend_types::parser::token::{is_year, is_mostly_numbers, cjk_numerals};

pub fn find_prev_token<F>(tokens: &[Token<'_>], pos: Option<usize>, mut pred: F) -> Option<usize>
where
    F: FnMut(&Token<'_>) -> bool
{
    let idx = pos.unwrap_or(tokens.len());
    tokens[..idx].iter().enumerate().rev()
        .find_map(|(idx, t)| pred(t).then_some(idx))
}
pub fn find_next_token<F>(tokens: &[Token<'_>], pos: usize, skip_self: bool, pred: F) -> Option<usize>
where
    F: FnMut(&Token<'_>) -> bool
{
    let idx = if skip_self { pos + 1 } else { pos };
    tokens[idx..].iter().position(pred)
        .map(|i| i + idx)
}

pub fn is_token_isolated(tokens: &[Token<'_>], idx: usize) -> bool {
    let Some(prev) = find_prev_token(tokens, Some(idx), |t| !matches!(t.kind, TokenKind::Delimiter(_))) else
        { return false; };
    if !tokens[prev].is_bracket() { return false; }

    let Some(next) = find_next_token(tokens, idx, true, |t| !matches!(t.kind, TokenKind::Delimiter(_))) else
            { return false; };
    tokens[next].is_bracket()
}
pub fn is_valid_ep(ep: &str) -> bool {
    !ep.is_empty() && ep.len() <= 4 && ep.bytes().all(|b| b.is_ascii_digit())
}
pub fn is_valid_cjk_ep(ep: &str) -> bool {
    if ep.is_ascii() { is_valid_ep(ep) }
    else {
        let cdpts = ep.chars().count();
        cdpts > 0 && cdpts <= 4 && ep.chars().all(|c| cjk_numerals().contains(&c))
    }
}

pub fn from_ordinal(token: TokenKind<'_>) -> Option<&str> {
    match token {
        TokenKind::Text(txt) => {
            match txt.to_lowercase().as_str() {
                "first" | "1st" => Some("1"),
                "second" | "2nd" => Some("2"),
                "third" | "3rd" => Some("3"),
                "fourth" | "4th" => Some("4"),
                "fivth" | "5th" => Some("5"),
                "sixth" | "6th" => Some("6"),
                "seventh" | "7th" => Some("7"),
                "eighth" | "8th" => Some("8"),
                "ninth" | "9th" => Some("9"),
                _ => None,
            }
        }
        _ => None
    }
}
pub fn from_month_name(token: TokenKind<'_>) -> Option<&str> {
    match token {
        TokenKind::Text(txt) => {
            match txt.to_lowercase().as_str() {
                "january"   | "jan"  => Some("1"),
                "february"  | "feb"  => Some("2"),
                "march"     | "mar"  => Some("3"),
                "april"     | "apr"  => Some("4"),
                "may"                => Some("5"),
                "june"      | "jun"  => Some("6"),
                "july"      | "jul"  => Some("7"),
                "august"    | "aug"  => Some("8"),
                "september" | "sep"  => Some("9"),
                "october"   | "oct"  => Some("10"),
                "november"  | "nov"  => Some("11"),
                "december"  | "dec"  => Some("12"),
                _ => None,
            }
        }
        _ => None
    }
}

pub fn find_pair_mut<T, F, S>(slice: &mut [T], first: F, second: S) -> Option<((usize, &mut T), (usize, &mut T))>
where
    F: FnMut(&T) -> bool,
    S: FnMut(&T) -> bool,
{
    let first = slice.iter().position(first)?;
    let off = first + 1;
    let second = off + slice.iter().skip(off).position(second)?;
    let (left, right) = slice.split_at_mut(second);
    Some(((first, &mut left[first]), (second, &mut right[0])))
}

pub fn combine_tokens(tokens: &[Token<'_>], keep_delimeters: bool) -> String {
    let delims = tokens.iter()
        .filter_map(|t| match t.kind {
            TokenKind::Delimiter(d) => d.chars().next(),
            _ => None
        })
        .collect::<Vec<_>>();
    let has_single_delim = delims.first()
        .map(|d| delims.iter().all(|x| x == d))
        .unwrap_or(false);
    let has_underscores = delims.contains(&'_');
    let has_spaces = delims.iter()
        .any(|&x| get_general_category(x) == GeneralCategory::SpaceSeparator);

    let is_transformable_delim = |tok: &Token<'_>| {
        if keep_delimeters { return false; }
        let TokenKind::Delimiter(d) = tok.kind else { return false; };
        let Some(ch) = d.chars().next() else { return false; };

        if ch == ',' || ch == '&' { false }
        else if ch == '_' || get_general_category(ch) == GeneralCategory::SpaceSeparator { true }
        else if has_underscores || has_spaces { false }
        else if ch == '.' { true }
        else { has_single_delim }
    };

    let mut buf = String::new();
    // collapse consecutive spaces
    let mut last_was_space = false;
    for tok in tokens {
        if is_transformable_delim(tok) {
            if !last_was_space {
                buf.push(' ');
                last_was_space = true;
            }
        } else {
            let (TokenKind::OpenPunc(t) | TokenKind::ClosePunc(t) | TokenKind::Text(t) | TokenKind::Numeric(t) | TokenKind::Delimiter(t)) = tok.kind else {
                return "".to_string();
            };
            buf.push_str(t);
            last_was_space = false;
        }
    }

    if !keep_delimeters {
        let trimmed = buf.trim_matches(|c|
            c == ' '
            || get_general_category(c) == GeneralCategory::DashPunctuation
        );
        let ptr = trimmed.as_ptr();
        let len = trimmed.len();

        // as per anitomy-rs: the invariants of pointer value and length
        // are upheld by the above function
        unsafe {
            let v = buf.as_mut_vec();
            std::ptr::copy(ptr, v.as_mut_ptr(), len);
            v.set_len(len);
        }
    }
    buf
}
