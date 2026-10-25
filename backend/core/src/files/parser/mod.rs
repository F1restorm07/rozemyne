// ripped from Rapptz/anitomy-rs
// and johnthreekay/chaptr

use std::borrow::Cow;
use std::cell::RefCell;
use std::range::Range;
use std::rc::Rc;
use std::sync::LazyLock;

use itertools::Itertools;
use regex::Regex;
use uncased::UncasedStr;
use unicode_general_category::{GeneralCategory, get_general_category};

use backend_types::parser::{Segment, SegmentKind, keyword_to_title_part};
use backend_types::parser::keyword::KeywordKind;
use backend_types::parser::token::{Token, TokenKind};

mod utils;
use utils::*;

static VIDEO_RES_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r#"^\d{3,4}(?:[ip]|[xX×]\d{3,4}[ip]?)$"#).unwrap()
);
static EPISODE_PRE_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r#"^(?:E|[Ee][Pp]|Eps)(\d{1,4}(?:\.5)?)(?:[vV](\d))?$"#).unwrap()
);
static SEASON_EP_REGEX: LazyLock<Regex> = LazyLock::new(||
    // case insensitive to match `s05e10`
    Regex::new(r#"(?i)^S?(\d{1,2})(?:-S?(\d{1,2}))?(?:x|[ ._-x]?EP?)(\d{1,4})(?:-?(?:EP?)?(\d{1,4}))?(?:[vV](\d))?$"#).unwrap()
);

const NON_ENGLISH_CHARS: &str = r"\p{Script=Han}\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Cyrillic}";
static WEBSITE_TYPE_1_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"^(www\.[\w-]+\.[\w-]+)\s+-\s*").unwrap()
);
static WEBSITE_TYPE_2_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"^((?:www\.)?[\w-]+\.[\w-]+(?:\.[\w-]+)?)\s+-\s*").unwrap()
);
// modified from "http://flanders.co.nz/2009/11/08/a-good-url-regular-expression-repost/" to match urls like "13DL.APP" in "13DL.APP-Tsukunechan + 30 v01-03.rar"
static WEBSITE_TYPE_3_REGEX: LazyLock<Regex> = LazyLock::new(||
    Regex::new(r"^(?:(?:ht|f)tp(?:s?)://|~/|/)?(?:\w+:\w+@)?(?:(?:[-\w]+\.)+(?:com|org|net|gov|mil|biz|info|mobi|name|aero|jobs|museum|travel|[a-zA-Z]{2,3}))(?::[\d]{1,5})?(?:(?:(?:/(?:[-\w~!$+|.,=]|%[a-f\d]{2})+)+|/)+|\?|#)?(?:(?:\?(?:[-\w~!$+|.,*:]|%[a-f\d{2}])+=?(?:[-\w~!$+|.,*:=]|%[a-f\d]{2})*)(?:&(?:[-\w~!$+|.,*:]|%[a-f\d{2}])+=?(?:[-\w~!$+|.,*:=]|%[a-f\d]{2})*)*)*(?:#(?:[-\w~!$+|.,*:=]|%[a-f\d]{2})*)?\b").unwrap()
);
static ALT_TITLES_ENCLOSED: LazyLock<Regex> = LazyLock::new(|| {
    // strip release groups
    // also includes latin alt titles wrapped in parenthesis (ex. "(LasTame)", "(Bron Broen)" )
    // since they can mess with future searches
    Regex::new(r"(.+)[ .]?[\[【★\(].*[\]】★\)]$").expect("could not compile russian cast regex")
});
static ALT_TITLES: LazyLock<Regex> = LazyLock::new(|| {
    // the \p{Emoji_Presentation} is to catch emoji which delimit titles (ex. `Champignon no Majo 🍄 シャンピニオンの魔女`)
    // and exclude matching `▽` in titles like `ピタゴラスイッチ▽じゃんけん装置 ターンテーブル ▽そこで橋は考えた`
    // \p{Emoji} is not used here as it includes characters `0-9`, breaking the cleaning
    let regex =
        format!(r"[^/|\p{{Emoji_Presentation}}(]*[{NON_ENGLISH_CHARS}][^/|\p{{Emoji_Presentation}}]*[/|\p{{Emoji_Presentation}}]|[/|\p{{Emoji_Presentation}}][^/|\p{{Emoji_Presentation}}(]*[{NON_ENGLISH_CHARS}][^/|\p{{Emoji_Presentation}}]*");
    Regex::new(&regex).expect("could not compile alt titles regex")
});
// regex for removing leftover bracketed items (ex. "One Piece 1164 [VERSION LIGHT][VOSTFR][FHD")
static REM_BRACKETED_ITEMS: LazyLock<Regex> = LazyLock::new(|| {
    let regex = format!(r"(?:[\[\]({{}}](?:[A-Za-z{NON_ENGLISH_CHARS}]+[ _.-]?)+[\[\]({{}}]?)+$");
    Regex::new(&regex).expect("could not compile remaining bracketed item regex")
});

// taken from Rapptz/anitomy-rs
pub fn parse(mut tokens: Vec<Token<'_>>) -> Vec<Segment<'_>> {
    let mut parts = Vec::new();

    if let Some(p) = parse_file_extension(&mut tokens) {
        parts.push(p);
    }
    parse_keywords(&mut tokens, &mut parts);
    parse_video_resolution(&mut tokens, &mut parts);
    if let Some(p) = parse_date(&mut tokens) {
        parts.push(p)
    }
    if let Some(p) = parse_year(&mut tokens) {
        parts.push(p);
    }
    parse_season(&mut tokens, &mut parts);
    parse_volume(&mut tokens, &mut parts);
    parse_episode(&mut tokens, &mut parts, SegmentKind::Episode);
    if let Some(t) = parse_title(&mut tokens, &parts) {
        parts.push(t);
    }
    if let Some(t) = parse_release_group(&mut tokens) {
        parts.push(t);
    }
    if let Some(t) = parse_episode_title(&mut tokens) {
        parts.push(t);
    }

    parts.sort_by_key(|t| t.pos.start);
    parts
}

fn parse_file_extension<'s>(tokens: &mut [Token<'s>]) -> Option<Segment<'s>> {
    let [prev, last] = tokens.last_chunk_mut().unwrap();

    let is_file_ext = last.keyword.as_ref()
        .is_some_and(|k| k.kind == KeywordKind::FileExtension);
    let is_dot = matches!(prev.kind, TokenKind::Delimiter("."));

    if is_file_ext && is_dot {
        let TokenKind::Text(txt) = last.kind else { return None; };
        prev.is_known = true;
        last.is_known = true;
        Some(Segment {
            kind: SegmentKind::FileExtension,
            text: Cow::Borrowed(txt),
            pos: last.span,
        })
    } else { None }
}

fn parse_keywords<'s>(tokens: &mut [Token<'s>], parts: &mut Vec<Segment<'s>>) {
    for token in tokens.iter_mut().filter(|t| t.is_free()) {
        let Some(ref keyword) = token.keyword else { continue; };
        let txt = match token.kind {
            TokenKind::Text(t) => t,
            TokenKind::Numeric(t) => t,
            _ => unreachable!()
        };

        if !keyword.is_ambiguous() || token.is_enclosed {
            token.is_known = true;
        }

        let txt = match keyword.kind {
            KeywordKind::ReleaseVersion => &txt[1..],
            _ => txt
        };
        parts.push(Segment {
            kind: keyword_to_title_part(keyword.kind.clone()),
            text: Cow::Borrowed(txt),
            pos: token.span,
        });
    }
}

fn parse_video_resolution<'s>(tokens: &mut [Token<'s>], parts: &mut Vec<Segment<'s>>) {
    let mut found_tok = parts.iter()
        .any(|e| e.kind == SegmentKind::Resolution);
    for token in tokens.iter_mut()
        .filter(|t| {
            let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = t.kind else { return false };
            VIDEO_RES_REGEX.is_match(txt)
        }) {
            let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = token.kind else
                { panic!("something has gone horribly wrong") };
            token.is_known = true;
            parts.push(Segment {
                kind: SegmentKind::Resolution,
                text: Cow::Borrowed(txt),
                pos: token.span,
            });
            found_tok = true;
    }

    if !found_tok
        && let Some(tok) = tokens.iter_mut()
            .find(|t| matches!(t.kind, TokenKind::Numeric("1080") | TokenKind::Numeric("720")))
        {
            // i should really extract the token's value into a separate field
            let TokenKind::Numeric(txt) = tok.kind else { panic!("uh-oh") };
            parts.push(Segment {
                kind: SegmentKind::Resolution,
                text: Cow::Borrowed(txt),
                pos: tok.span,
            });
        }
}

fn parse_date<'s>(tokens: &mut [Token<'s>]) -> Option<Segment<'s>> {
    // wrap the tokens to allow for cloning and dynamic borrowing
    let iter = tokens.iter_mut().map(|t| Rc::new(RefCell::new(t)));
    // parsing date in format `yyyy.mm.dd`, where the tokens show up as [`yyyy`.`mm`.`dd`]
    for (year, delim1, month, delim2, day) in iter.tuple_windows() {

        if !(matches!(delim1.borrow().kind, TokenKind::Delimiter(d) if d.starts_with(['.', '-']))
            && matches!(delim2.borrow().kind, TokenKind::Delimiter(d) if d.starts_with(['.', '-'])))
        {
            continue;
        }

        let TokenKind::Numeric(y) = year.borrow().kind else { continue; };
        let TokenKind::Numeric(m) = month.borrow().kind else { continue; };
        let TokenKind::Numeric(d) = day.borrow().kind else { continue; };

        if let Ok(date) =
            jiff::civil::Date::new(
                y.parse().unwrap_or(i16::MAX),
                m.parse().unwrap_or(i8::MAX),
                d.parse().unwrap_or(i8::MAX)
            )
        {
            year.borrow_mut().is_known = true;
            delim1.borrow_mut().is_known = true;
            month.borrow_mut().is_known = true;
            delim2.borrow_mut().is_known = true;
            day.borrow_mut().is_known = true;
            return Some(Segment {
                kind: SegmentKind::Date,
                text: Cow::Owned(date.to_string()),
                pos: Range::from(year.borrow().span.start..day.borrow().span.end)
            });
        }
    }
    // remake the iterator as i cannot exactly clone it
    let iter = tokens.iter_mut().map(|t| Rc::new(RefCell::new(t)));
    // parse date in format of `<ordinal day> <month name> <year>`
    // @todo: also accept numeric days, and the format `<month> <day> <year>`
    for (day, delim1, month, delim2, year) in iter.tuple_windows() {
        let Some(d) = from_ordinal(day.borrow().kind.clone()) else { continue; };
        let Some(m) = from_month_name(month.borrow().kind.clone()) else { continue; };
        let y = match year.borrow().kind {
            TokenKind::Numeric(y) if is_year(y.parse().unwrap_or(i16::MAX)) => y,
            _ => continue,
        };

        if let Ok(date) =
            jiff::civil::Date::new(
                y.parse().unwrap_or(i16::MAX),
                m.parse().unwrap_or(i8::MAX),
                d.parse().unwrap_or(i8::MAX)
            )
        {
            year.borrow_mut().is_known = true;
            delim1.borrow_mut().is_known = true;
            month.borrow_mut().is_known = true;
            delim2.borrow_mut().is_known = true;
            day.borrow_mut().is_known = true;
            return Some(Segment {
                kind: SegmentKind::Date,
                text: Cow::Owned(date.to_string()),
                pos: Range::from(year.borrow().span.start..day.borrow().span.end)
            });
        }
    }

    let iter = tokens.iter_mut().map(|t| Rc::new(RefCell::new(t)));
    // parse date in format of `Week <num> <year>`
    for (week, delim1, num, delim2, year) in iter.tuple_windows() {
        if !matches!(week.borrow().kind, TokenKind::Text(w) if w.eq_ignore_ascii_case("week"))
            { continue; }
        let TokenKind::Numeric(w) = num.borrow().kind else { continue; };
        let y = match year.borrow().kind {
            TokenKind::Numeric(y) if is_year(y.parse().unwrap_or(i16::MAX)) => y,
            _ => continue,
        };

        if let Ok(date) =
            jiff::civil::ISOWeekDate::new(
                y.parse().unwrap_or(i16::MAX),
                w.parse().unwrap_or(i8::MAX),
                // always use monday as we don't get the weekday number here
                jiff::civil::Weekday::Monday,
            )
        {
            week.borrow_mut().is_known = true;
            delim1.borrow_mut().is_known = true;
            num.borrow_mut().is_known = true;
            delim2.borrow_mut().is_known = true;
            year.borrow_mut().is_known = true;
            return Some(Segment {
                kind: SegmentKind::Date,
                text: Cow::Owned(date.date().to_string()),
                pos: Range::from(week.borrow().span.start..year.borrow().span.end)
            });
        }
    }
    None
}

fn parse_year<'s>(tokens: &mut [Token<'s>]) -> Option<Segment<'s>> {
    // enclosed year (e.g. `(2020)`)
    if let Some(tok) = tokens.windows(3)
        .enumerate()
        .find(|(_, x)| {
            matches!(x[0].kind, TokenKind::OpenPunc(_))
                && matches!(x[2].kind, TokenKind::ClosePunc(_))
                && matches!(x[1].kind, TokenKind::Numeric(t) if is_year(t.parse().unwrap_or(i16::MAX)))
        })
        // avoid multiple borrows of `tokens`
        .map(|(i, _)| i + 1)
        .and_then(|idx| tokens.get_mut(idx))
    {
        tok.is_known = true;
        let TokenKind::Numeric(txt) = tok.kind else { return None; };
        return Some(Segment {
            kind: SegmentKind::Year,
            text: Cow::Borrowed(txt),
            pos: tok.span,
        })
    }

    for idx in tokens.iter()
        .enumerate()
        .filter(|(_, x)|
            !x.is_enclosed
            && matches!(x.kind, TokenKind::Numeric(t) if is_year(t.parse().unwrap_or(i16::MAX)))
        )
        .map(|(i, _)| i)
    {
        if is_token_isolated(tokens, idx) {
            tokens[idx].is_known = true;
            let TokenKind::Numeric(t) = tokens[idx].kind else { return None; };
            return Some(Segment {
                kind: SegmentKind::Year,
                text: Cow::Borrowed(t),
                pos: tokens[idx].span,
            });
        }
    }

    // check for the last possible freestanding year (e.g. `The.Legend.of.1900.1998`)
    // search from the end to prevent matching titles which contain years (e.g. `2001 A Space Odyssey`)
    // this causes problems for year ranges (i keep the tokens separate to avoid weird episode matches) (e.g. `Let It Fall Los Angeles 1982-1992 2017`)
    if let Some(y) = find_prev_token(tokens, None, |t| matches!(t.kind, TokenKind::Numeric(n) if is_year(n.parse().unwrap_or(i16::MAX)))) {
        // prevent matching the first token
        if y == 0 { return None; }
        tokens[y].is_known = true;
            let TokenKind::Numeric(t) = tokens[y].kind else { return None; };
            return Some(Segment {
                kind: SegmentKind::Year,
                text: Cow::Borrowed(t),
                pos: tokens[y].span,
            });
    }

    None
}

fn parse_season<'s>(tokens: &mut [Token<'s>], parts: &mut Vec<Segment<'s>>) {
    let token_windows = tokens.iter_mut().map(|t| Rc::new(RefCell::new(t)));
    for (before, delim, after) in token_windows.tuple_windows() {
        if after.borrow().keyword.as_ref().is_some_and(|k| k.kind == KeywordKind::Season)
            && matches!(delim.borrow().kind, TokenKind::Delimiter(_))
            && before.borrow().is_free()
        {
            let mut before = before.borrow_mut();
            if let Some(num) = from_ordinal(before.kind.clone()) {
                after.borrow_mut().is_known = true;
                delim.borrow_mut().is_known = true;
                before.is_known = true;
                parts.push(Segment {
                    kind: SegmentKind::Season,
                    text: Cow::Borrowed(num),
                    pos: Range::from(before.span.start..after.borrow().span.end),
                });
                return;
            }
        }
        if before.borrow().keyword.as_ref().is_some_and(|k| k.kind == KeywordKind::Season)
            && matches!(delim.borrow().kind, TokenKind::Delimiter(_))
            && after.borrow().is_free()
        {
            let mut after = after.borrow_mut();
            let value = match after.kind {
                TokenKind::Numeric(num) => Cow::Borrowed(num),
                TokenKind::Text(txt) => match txt.parse::<roman_numerals_rs::RomanNumeral>() {
                    Ok(num) => Cow::Owned(format!("{}", num.as_u16())),
                    // this could be a season range (ex. `Season 1-4`)
                    Err(_) => {
                        if let TokenKind::Text(txt) = after.kind {
                            if let Some((first, last)) = txt.split_once(['.', '-']) {
                                if first.parse::<u16>().and(last.parse::<u16>()).is_ok() {
                                    parts.push(Segment {
                                        kind: SegmentKind::Season,
                                        text: Cow::Borrowed(first),
                                        pos: after.span,
                                    });
                                    parts.push(Segment {
                                        kind: SegmentKind::Season,
                                        text: Cow::Borrowed(last),
                                        pos: after.span,
                                    });
                                    after.is_known = true;
                                    break;
                                } else { continue; }
                            } else { continue; }
                        } else { continue; }
                    }
                }
                _ => unreachable!()
            };
            after.is_known = true;
            delim.borrow_mut().is_known = true;
            before.borrow_mut().is_known = true;
            parts.push(Segment {
                kind: SegmentKind::Season,
                text: value,
                pos: Range::from(before.borrow().span.start..after.span.end),
            });
            break;
            // fall through here as many filenames duplicate season entries
            // (ex. `Family.Guy.S17.Complete.Season.17.x264.720p`)
            // return;
        }
    }

    for token in tokens.iter_mut().filter(|t| t.is_free()) {
        let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = token.kind else {
            return;
        };

        for val in txt.split(['.', '-', '&', '+', '~']) {
            if let Some(suf) = val.strip_prefix(['S', 's'])
                && (suf.len() == 1 || suf.len() == 2) && suf.chars().all(|c| c.is_ascii_digit()) {
                    token.is_known = true;
                    parts.push(Segment {
                        kind: SegmentKind::Season,
                        text: Cow::Borrowed(suf),
                        pos: token.span,
                    });
                }
            if let Some(pre) = val.strip_suffix('期') {
                let pre = pre.strip_prefix('第').unwrap_or(pre);
                if (pre.len() == 1 || pre.len() == 2) && pre.chars().all(|c| c.is_ascii_digit()) {
                    token.is_known = true;
                    parts.push(Segment {
                        kind: SegmentKind::Season,
                        text: Cow::Borrowed(pre),
                        pos: token.span,
                    });
                }
            }
        }
    }
}

// taken from johnthreekay/chaptr
fn bare_number_ep<'s>(tokens: &mut [Token<'s>], parts: &mut Vec<Segment<'s>>, kind: SegmentKind) {
    let first_vol_marker = find_next_token(
        tokens,
        0,
        true,
        |t| t.keyword.as_ref()
            .is_some_and(|k| matches!(k.kind, KeywordKind::Volume | KeywordKind::Season))
    );
    let min_index = first_vol_marker.map_or(0, |p| p + 1);

    for i in 0..tokens.len() {
        if !tokens[i].is_free() { continue; }

        match tokens[i].kind {
            TokenKind::Numeric(txt) => {
                let vol_sea_kywd = find_prev_token(
                    tokens,
                    Some(i),
                    |t| t.keyword.as_ref()
                        .is_some_and(|k|
                            matches!(k.kind,
                                KeywordKind::Volume | KeywordKind::Season
                                | KeywordKind::Chapter | KeywordKind::Part
                            )
                        )
                );
                if i >= min_index
                    // skip if the previous token is "movie" ("part" is covered with the `Part` keyword)
                    && find_prev_token(tokens, Some(i), |t| matches!(t.kind, TokenKind::Text(t) if t != UncasedStr::new("Movie"))).is_some()
                    && vol_sea_kywd.is_none()
                    && !is_year(txt.parse().unwrap_or(i16::MAX))
                    && is_number_followed_by_metadata(tokens, i + 1)
                    && (is_valid_ep(txt) || is_valid_cjk_ep(txt)) {
                        tokens[i].is_known = true;
                        parts.push(Segment {
                            kind,
                            text: Cow::Borrowed(txt),
                            pos: tokens[i].span,
                        });
                        return;
                    }
            }
            TokenKind::Text(txt) => {
                let vol_sea_kywd = find_prev_token(
                    tokens,
                    Some(i),
                    |t| t.keyword.as_ref()
                        .is_some_and(|k| matches!(k.kind, KeywordKind::Volume | KeywordKind::Season))
                );

                // @todo: parse a range here, too
                // alpha suffix case: `153b`
                if i >= min_index
                    && find_prev_token(tokens, Some(i), |t| matches!(t.kind, TokenKind::Text(_))).is_some()
                    && vol_sea_kywd.is_none()
                    && !is_year(txt.parse().unwrap_or(i16::MAX))
                    && is_number_followed_by_metadata(tokens, i + 1)
                {
                    let digits_end = txt.bytes().take_while(u8::is_ascii_digit).count();
                    if digits_end > 0 && !is_year(txt[..digits_end].parse().unwrap_or(i16::MAX)) {
                        if txt.get(..digits_end).is_some_and(|t| t.parse::<u32>().is_ok()) {
                            let suffix = txt.get(digits_end..);
                            if suffix.is_some_and(|t| t.is_empty()) {
                                tokens[i].is_known = true;
                                parts.push(Segment {
                                    kind,
                                    text: Cow::Borrowed(&txt[..digits_end]),
                                    pos: tokens[i].span,
                                });
                                return;
                            }
                            if suffix.is_some_and(|t| t.len() == 1 && t.as_bytes()[0].is_ascii_lowercase()) {
                                tokens[i].is_known = true;
                                parts.push(Segment {
                                    kind,
                                    text: Cow::Borrowed(txt),
                                    pos: tokens[i].span,
                                });
                                return;
                            }
                        }
                        tokens[i].is_known = true;
                        parts.push(Segment {
                            kind,
                            text: Cow::Borrowed(&txt[..digits_end]),
                            pos: tokens[i].span,
                        });
                    }
                }
            }
            _ => {}

        }
    }
}

// taken from johnthreekay/chaptr
fn is_number_followed_by_metadata(tokens: &[Token<'_>], start_idx: usize) -> bool {
    // does this number look like an episode or chapter
    if let Some(Token { kind: TokenKind::Delimiter(d), .. }) = tokens.get(start_idx)
        && let Some(Token { kind: n_kind @ (TokenKind::Numeric(txt) | TokenKind::Text(txt)), keyword, .. }) = tokens.get(start_idx + 1)
    {
        // do not match on numbers right before years (these are most likely part of the title)
        // ex. `Shrek.2.2004.1080p.BluRay.x264.YIFY.srt`
        if matches!(n_kind, TokenKind::Numeric(_))
            && let Some(next) = find_next_token(tokens, start_idx, true, |t| !matches!(t.kind, TokenKind::Delimiter(_)))
            && let TokenKind::Numeric(y) = tokens[next].kind
            && is_year(y.parse().unwrap_or(i16::MAX))
        {
            return false;
        }

        match *d {
            "." if matches!(n_kind, TokenKind::Numeric(_))
                || keyword.as_ref().is_some_and(|k| k.kind == KeywordKind::FileExtension)
                => return true,
            "-" => {
                let digits_end = txt.bytes().take_while(u8::is_ascii_digit).count();
                if digits_end > 0 {
                    if txt.get(..digits_end).is_some_and(|t| t.parse::<u32>().is_ok()) {
                        let suffix = txt.get(digits_end..);
                        if suffix.is_some_and(|t| t.is_empty()) {
                            return true;
                        }
                        if suffix.is_some_and(|t| t.len() == 1 && t.as_bytes()[0].is_ascii_lowercase()) {
                            return true;
                        }
                    }
                    return true;
                }
            }
            _ => {}
        }
    }

    for tok in &tokens[start_idx.min(tokens.len())..] {
        match tok.kind {
            TokenKind::Delimiter(_) => continue,
            TokenKind::OpenPunc(_) | TokenKind::ClosePunc(_) => return true,
            TokenKind::Text(_) =>
                return tok.keyword.as_ref()
                    .is_some_and(|k| matches!(k.kind,
                            KeywordKind::Chapter | KeywordKind::Volume
                            | KeywordKind::Episode | KeywordKind::Season
                            | KeywordKind::Subbed | KeywordKind::Dubbed
                    )),
            // do not match on numbers right before years (these are most likely part of the title)
            TokenKind::Numeric(n) =>
                return !is_year(n.parse().unwrap_or(i16::MAX)),
            _ => {}
        }
    }
    true
}

// @todo: toggle this via an option flag (i only want to parse for this on manga)
// taken from johnthreekay/chaptr
fn parse_volume<'s>(tokens: &mut [Token<'s>], parts: &mut Vec<Segment<'s>>) {
    // volume prefixes and postfixes (e.g. `Vol.1`, `Volume 1`, `5 Том`, etc)
    for idx in 0..tokens.len() {
        if !tokens[idx].keyword.as_ref().is_some_and(|k| k.kind == KeywordKind::Volume) { continue; }

        // check for prefixes first
        let Some(next) = find_next_token(tokens, idx, true, |t| !matches!(t.kind, TokenKind::Delimiter(_))) else {
            // now check for postfixes
            let Some(prev) = find_prev_token(tokens, Some(idx), |t| !matches!(t.kind, TokenKind::Delimiter(_))) else {
                continue;
            };
            if !tokens[prev].is_free() { continue; }

            if parse_multi_ep_range(tokens, idx, parts, SegmentKind::VolumeRange) {
                tokens[idx].is_known = true;
                tokens[prev].is_known = true;
                continue;
            }

            let Some((pre, suf)) = parse_single_episode(tokens[prev].kind.clone()) else { continue; };
            parts.push(Segment {
                kind: SegmentKind::Volume,
                text: Cow::Borrowed(pre),
                pos: tokens[idx].span
            });
            parts.push(Segment {
                kind: SegmentKind::Version,
                text: Cow::Borrowed(suf),
                pos: tokens[idx].span
            });
            return;
        };
        if !tokens[next].is_free() { continue; }

        if parse_multi_ep_range(tokens, idx, parts, SegmentKind::VolumeRange) {
            tokens[idx].is_known = true;
            tokens[next].is_known = true;
            continue;
        }

        let Some((pre, suf)) = parse_single_episode(tokens[next].kind.clone()) else { continue; };
        parts.push(Segment {
            kind: SegmentKind::Volume,
            text: Cow::Borrowed(pre),
            pos: tokens[idx].span
        });
        parts.push(Segment {
            kind: SegmentKind::Version,
            text: Cow::Borrowed(suf),
            pos: tokens[idx].span
        });
    }

    // volume marker (e.g. `v01-18`)
    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        let TokenKind::Text(txt) = tok.kind else { continue; };

        if let Some(suf) = txt.strip_prefix(['v', 't']) {
            if let Some((first, last)) = suf.split_once(['-', '.']) {
                if is_valid_ep(first) && is_valid_ep(last) {
                    parts.push(Segment {
                        kind: SegmentKind::VolumeRange,
                        text: Cow::Borrowed(first),
                        pos: tok.span,
                    });
                    parts.push(Segment {
                        kind: SegmentKind::VolumeRange,
                        text: Cow::Borrowed(last),
                        pos: tok.span,
                    });
                    tok.is_known = true;
                    return;
                }
            } else {
                if is_valid_ep(suf) {
                    tok.is_known = true;
                    parts.push(Segment {
                        kind: SegmentKind::Volume,
                        text: Cow::Borrowed(suf),
                        pos: tok.span,
                    });
                    return;
                }
            }
        }
    }

    // cjk markers (e.g. `第01-02巻`)
    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        let TokenKind::Text(txt) = tok.kind else { continue; };

        // @todo: parse multi-character prefixes (e.g. `시즌3`)
        // @todo: catch `38-1화` case (<- chapter 38, part 1)
        if let Some(pre) = txt.strip_suffix(['巻', '卷', '册', '권', '장']) {
            let pre = pre.strip_prefix('第').unwrap_or(pre);

            // @todo: switch this out for a match statement
            if let Some((first, last)) = pre.split_once(['-', '.']) {
                if is_valid_cjk_ep(first) && is_valid_cjk_ep(last) {
                    parts.push(Segment {
                        kind: SegmentKind::Volume,
                        text: Cow::Borrowed(first),
                        pos: tok.span,
                    });
                    parts.push(Segment {
                        kind: SegmentKind::Volume,
                        text: Cow::Borrowed(last),
                        pos: tok.span,
                    });
                    tok.is_known = true;
                    return;
                }
            } else {
                if is_valid_cjk_ep(pre) {
                    tok.is_known = true;
                    parts.push(Segment {
                        kind: SegmentKind::Volume,
                        text: Cow::Borrowed(pre),
                        pos: tok.span,
                    });
                    return;
                }
            }
        }
    }
}

// @todo: finish porting this function over from anitomy-rs
fn parse_episode<'s>(tokens: &mut [Token<'s>], parts: &mut Vec<Segment<'s>>, kind: SegmentKind) {
    let _is_reg_ep = kind == SegmentKind::Episode;

    // episode prefixes (e.g. `EP1`, `Episode 1`)
    for idx in 0..tokens.len() {
        if !tokens[idx].is_free() { continue; }

        let is_keyword = tokens[idx].keyword.as_ref()
            .is_some_and(|k| k.kind == KeywordKind::Episode);
        if is_keyword
            && let Some(next) = find_next_token(tokens, idx, true, |t| !matches!(t.kind, TokenKind::Delimiter(_)))
                && tokens[next].is_free()
                    && is_mostly_numbers(tokens[next].kind.clone())
                {
                    if parse_multi_ep_range(tokens, next, parts, kind) {
                        return;
                    }

                    if let TokenKind::Numeric(num) = tokens[next].kind {
                        tokens[idx].is_known = true;
                        tokens[next].is_known = true;
                        parts.push(Segment {
                            kind: SegmentKind::Episode,
                            text: Cow::Borrowed(num),
                            pos: tokens[next].span,
                        });
                    }
                }
    }

    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = tok.kind else { continue; };
        if let Some(ep) = EPISODE_PRE_REGEX.captures(txt) {
            parts.push(Segment {
                kind,
                text: Cow::Borrowed(ep.get(1).unwrap().as_str()),
                pos: tok.span,
            });
            tok.is_known = true;
            if let Some(ver) = ep.get(2) {
                parts.push(Segment {
                    kind,
                    text: Cow::Borrowed(ver.as_str()),
                    pos: tok.span,
                });
            }
            return;
        }
    }

    // number in set (e.g. `8 of 12`)
    if let Some(ep) = parse_num_in_num_ep(tokens) {
        parts.push(ep);
        return;
    }

    // single episode (e.g. `01v2`)
    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        if let Some((ep, ver)) = parse_single_episode(tok.kind.clone())
            && !ver.is_empty() {
                tok.is_known = true;
                parts.push(Segment {
                    kind,
                    text: Cow::Borrowed(ep),
                    pos: tok.span,
                });
                parts.push(Segment {
                    kind: SegmentKind::Version,
                    text: Cow::Borrowed(ver),
                    pos: tok.span,
                });
                return;
            }
    }

    // multi-episodes (e.g. `01-04`)
    for idx in 0..tokens.len() {
        if tokens[idx].is_free() && parse_multi_ep_range(tokens, idx, parts, SegmentKind::EpisodeRange)
        { continue; }
    }

    // season and episode (e.g. `S01E03`, `1x03`)
    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = tok.kind else { return; };
        if let Some(caps) = SEASON_EP_REGEX.captures(txt) {
            // also match on season 0 (ex. `Deadliest.Catch.S00E66.No.Safe.Passage.720p...`)
            // this shouldn't cause any problems unless something weird comes up
            parts.push(Segment {
                kind: SegmentKind::Season,
                text: Cow::Borrowed(caps.get(1).unwrap().as_str()),
                pos: tok.span,
            });
            tok.is_known = true;
            if let Some(inner) = caps.get(2) {
                parts.push(Segment {
                    kind: SegmentKind::Season,
                text: Cow::Borrowed(inner.as_str()),
                pos: tok.span,
                });
            }

            parts.push(Segment {
                kind,
                text: Cow::Borrowed(caps.get(3).unwrap().as_str()),
                pos: tok.span,
            });
            if let Some(inner) = caps.get(4) {
                parts.push(Segment {
                kind,
                text: Cow::Borrowed(inner.as_str()),
                pos: tok.span,
                });
            }
            if let Some(inner) = caps.get(5) {
                parts.push(Segment {
                kind: SegmentKind::Version,
                text: Cow::Borrowed(inner.as_str()),
                pos: tok.span,
                });
            }
            return;
        }
    }

    // cjk counters (e.g. `第01話`)
    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = tok.kind else { return; };
        if let Some(pre) = txt.strip_suffix(['話', '话']) {
            let pre = pre.strip_prefix('第').unwrap_or(pre);
            if is_valid_cjk_ep(pre) {
                tok.is_known = true;
                parts.push(Segment {
                    kind,
                    text: Cow::Borrowed(pre),
                    pos: tok.span,
                });
                return;
            }
        }
    }

    // separated number `- 08`
    for idx in 0..tokens.len() {
        let is_valid = {
            let tok = &tokens[idx];
            // attempts to match the delimeter of ` - ` as well as `-`
            // this should work as the multi episodes (e.g. `01-10`) have already been matched above
            matches!(tok.kind, TokenKind::Delimiter(d) if d.chars().any(|c| get_general_category(c) == GeneralCategory::DashPunctuation))
        };
        if !is_valid {
            continue;
        }

        if let Some(tok) = tokens.iter_mut().skip(idx).find(|t| !matches!(t.kind, TokenKind::Delimiter(_)))
            && let TokenKind::Numeric(num) = tok.kind {
                tok.is_known = true;
                parts.push(Segment {
                    kind,
                    text: Cow::Borrowed(num),
                    pos: tok.span,
                });
                tokens[idx].is_known = true;
                return;
            }
    }

    // fractional episodes (e.g. `7.5`)
    for tok in tokens.iter_mut().filter(|t| t.is_free()) {
        let (TokenKind::Text(txt) | TokenKind::Numeric(txt)) = tok.kind else { return; };
        if let Some((first, second)) = txt.split_once('.')
            && second == "5" && is_valid_ep(first) {
                tok.is_known = true;
                parts.push(Segment {
                    kind,
                    text: Cow::Borrowed(txt),
                    pos: tok.span,
                });
                return;
            }
    }

    // isolated number (e.g. `[12]`, `(2006)`)
    {
        let iter = tokens.iter_mut().map(|t| Rc::new(RefCell::new(t)));
        for (before, mid, after) in iter.tuple_windows() {
            let mut mid = mid.borrow_mut();
            if matches!(before.borrow().kind, TokenKind::OpenPunc(_))
                && matches!(after.borrow().kind, TokenKind::ClosePunc(_))
                && mid.is_free()
                && let TokenKind::Numeric(num) = mid.kind
            {
                parts.push(Segment {
                    kind,
                    text: Cow::Borrowed(num),
                    pos: mid.span,
                });
                mid.is_known = true;
                return;
            }
        }
    }

    // freestanding numbers (e.g. `[Kaerizaki-Fansub]_One_Piece_1164_[VERSION_LIGHT][VOSTFR][FHD_1920x1080].mp4`)
    // this also breaks titles like `John Wick Chapter 2`, `Shrek 2`, and `The Twilight Saga Breaking Dawn - Part 2`
    bare_number_ep(tokens, parts, SegmentKind::Episode);
}

fn parse_single_episode(token: TokenKind<'_>) -> Option<(&str, &str)> {
    match token {
        TokenKind::Text(txt) => match txt.split_once(['V', 'v']) {
            Some((ep, vol)) => {
                if is_valid_ep(ep) && vol.len() == 1 && vol.as_bytes()[0].is_ascii_digit() {
                    Some((ep, vol))
                } else { None }
            }
            None if is_valid_ep(txt) => Some((txt, "")),
            None => None,
        }
        TokenKind::Numeric(num) if is_valid_ep(num) => Some((num, "")),
        _ => None,
    }
}

fn parse_multi_ep_range<'s>(
    tokens: &mut [Token<'s>],
    idx: usize,
    parts: &mut Vec<Segment<'s>>,
    kind: SegmentKind
) -> bool {
    let TokenKind::Text(txt) = tokens[idx].kind else { return false; };

    // splitting on the period can cause problems if non-episode tokens end up here
    // (e.g. `The.Legend.of.1900.1998` -> `1900.1998` -> (`1900`, `1998`))
    if let Some((first, last)) = txt.split_once(['-', '.', '+', '&']) {
        let tok = &mut tokens[idx];

        if let Some(((low, low_v), (up, up_v))) =
            parse_single_episode(TokenKind::Text(first)).zip(parse_single_episode(TokenKind::Text(last))) {
            match low.parse::<u16>().ok().zip(up.parse::<u16>().ok()) {
                Some((x, y)) if x < y && (!is_year(low.parse().unwrap()) && !is_year(low.parse().unwrap())) => {
                    parts.push(Segment {
                        kind,
                        text: Cow::Borrowed(low),
                        pos: tok.span,
                    });
                    tok.is_known = true;
                    if !low_v.is_empty() {
                        parts.push(Segment {
                            kind: SegmentKind::Version,
                            text: Cow::Borrowed(low_v),
                            pos: tok.span,
                        });
                    }
                    parts.push(Segment {
                        kind,
                        text: Cow::Borrowed(up),
                        pos: tok.span,
                    });
                    if !up_v.is_empty() {
                        parts.push(Segment {
                            kind: SegmentKind::Version,
                            text: Cow::Borrowed(up_v),
                            pos: tok.span,
                        });
                    }
                }
                _ => {},
            }
        }
    }
    false
}

fn parse_num_in_num_ep<'s>(tokens: &mut [Token<'s>]) -> Option<Segment<'s>> {
    for idx in 0..tokens.len() {
        // for `Needy Girl Overdose | Зависимая девушка: Передозировка [TV-1] [2026] [09 of 12] [WEBRip] [1080p] [RUS + JAP]`
        // where `2026` matches both the year and the episode (find_next_token passes through the `] [` delimeter)
        if !tokens[idx].is_free() { continue; }
        let TokenKind::Numeric(num) = tokens[idx].kind else { continue; };

        let Some(mid) = find_next_token(tokens, idx, true, |t| {
            matches!(t.kind, TokenKind::Text("of") | TokenKind::Delimiter("&"))
        }) else { continue; };

        if let Some(other) = tokens[mid..].iter_mut().skip(1)
            .find(|t| !matches!(t.kind, TokenKind::Delimiter(_)))
            && other.is_numeric()
        {
            other.is_known = true;
            tokens[mid].is_known = true;
            tokens[idx].is_known = true;
            return Some(Segment {
                kind: SegmentKind::Episode,
                text: Cow::Borrowed(num),
                pos: tokens[idx].span,
            });
        }
    }
    None
}

fn parse_title<'s>(tokens: &mut [Token<'s>], parts: &[Segment<'_>]) -> Option<Segment<'s>> {
    let range = find_title(tokens)?;
    let title = combine_tokens(range, false);

    if title.is_empty() {
        None
    } else {
        let title = process_title(&title, parts);
        let start = range.first()?.span;
        let end = range.last()?.span;
        for tok in range {
            tok.is_known = true;
        }
        Some(Segment {
            kind: SegmentKind::Title,
            text: Cow::Owned(title),
            pos: Range::from(start.start..end.end),
        })
    }
}

fn find_title<'a, 's>(tokens: &'a mut [Token<'s>]) -> Option<&'a mut [Token<'s>]> {
    let mut first = tokens.iter().position(|t| !t.is_enclosed && t.is_free() && !t.is_known);
    let mut last =
        first.and_then(|idx| find_next_token(tokens, idx, true, |t| t.is_known));

    if first.is_none()
        && let Some((opp, idx)) = find_pair_mut(
            tokens,
            |t| matches!(t.kind, TokenKind::ClosePunc(_)),
            |t| matches!(t.kind, TokenKind::OpenPunc(_))
        ).and_then(|(_, (pos, open))| {
            let TokenKind::OpenPunc(p) = open.kind else { return None; };
            p.chars().next()
                .and_then(unicode_bidi_mirroring::get_mirrored)
                .zip(Some(pos))
        }) {
            first = find_next_token(tokens, idx, false, |t| t.is_free());
            last = first.and_then(|idx| {
                find_next_token(tokens, idx, true, |t| matches!(t.kind, TokenKind::ClosePunc(p) if p.starts_with(opp)))
            });
        }
    let index = first?;

    let slice = &tokens[index..last.unwrap_or(tokens.len())];
    let (count, last_idx) = slice.iter().enumerate()
        .filter_map(|(idx, tok)| matches!(tok.kind, TokenKind::OpenPunc(_)).then_some(idx))
        .fold((0, 0), |acc, x| (acc.0 + 1, x));
    if count != 0 {
        let closed_count = slice.iter().filter(|t| matches!(t.kind, TokenKind::ClosePunc(_))).count();
        if closed_count != count {
            last = Some(last_idx + index);
        }
    }

    if let Some(idx) = find_prev_token(tokens, last, |t| !matches!(t.kind, TokenKind::Delimiter(_))) {
        let token = &tokens[idx];
        if matches!(token.kind, TokenKind::Delimiter(d) if d != ")")
            && let Some(n_last) = find_prev_token(tokens, Some(idx), |t| matches!(t.kind, TokenKind::OpenPunc(_))) {
                last = Some(n_last);
            }
    }

    match (first, last) {
        (Some(f), Some(l)) => Some(&mut tokens[f..l]),
        (Some(f), None) => Some(&mut tokens[f..]),
        _ => None
    }
}

fn find_release_group<'a, 's>(tokens: &'a mut [Token<'s>]) -> Option<&'a mut [Token<'s>]> {
    let mut first = tokens.iter().position(|t| t.is_enclosed && !t.is_known);
    let mut last = get_last_idx_for_rel_group(tokens, first);

    while let Some((start, end)) = first.zip(last) {
        if start > tokens.len() || end > tokens.len() {
            break;
        }
        if tokens[start..end].iter().all(|t| !t.is_known) { break; }

        first = find_next_token(tokens, end, true, |t| t.is_enclosed && t.is_free());
        last = get_last_idx_for_rel_group(tokens, first);
    }

    if first.is_none()
        && let Some(idx) = find_prev_token(tokens, Some(tokens.len()), |t| {
            t.is_free() && !matches!(t.kind, TokenKind::Delimiter(_))
        })
    {
        let tok = &tokens[idx];
        if tok.is_free() && idx != 0
            && tokens.get(idx - 1).is_some_and(|t| matches!(t.kind, TokenKind::Delimiter("-")))
        {
            first = Some(idx);
            last = Some(idx + 1);
        }
    }

    match (first, last) {
        (Some(f), Some(l)) => Some(&mut tokens[f..l]),
        (Some(f), None) => Some(&mut tokens[f..]),
        _ => None,
    }
}

fn get_last_idx_for_rel_group<'t, 's>(tokens: &'t mut [Token<'s>], first: Option<usize>) -> Option<usize> {
    let other_bracket = find_prev_token(tokens, first, |t| !t.is_enclosed && matches!(t.kind, TokenKind::OpenPunc(_)))
        .and_then(|i| {
            let TokenKind::OpenPunc(punc) = tokens[i].kind else { return None; };
            punc.chars().next().and_then(unicode_bidi_mirroring::get_mirrored)
        });

    first.and_then(|idx| match other_bracket {
        Some(b) => find_next_token(tokens, idx, true, |t| {
            matches!(t.kind, TokenKind::ClosePunc(p) if p.starts_with(b))
        }),
        None => find_next_token(tokens, idx, true, |t| matches!(t.kind, TokenKind::ClosePunc(_)))
    })
}

fn parse_release_group<'s>(tokens: &mut [Token<'s>]) -> Option<Segment<'s>> {
    let range = find_release_group(tokens)?;
    let val = combine_tokens(range, true);

    if val.is_empty() {
        None
    } else {
        let start = range.first()?.span.start;
        let end = range.last()?.span.end;
        for tok in range {
            tok.is_known = true;
        }
        Some(Segment {
            kind: SegmentKind::Group,
            text: val.into(),
            pos: Range::from(start..end),
        })
    }
}

fn parse_episode_title<'s>(tokens: &mut [Token<'s>]) -> Option<Segment<'s>> {
    let range = find_episode_title(tokens)?;
    let value = combine_tokens(range, false);

    if value.is_empty() {
        None
    } else {
        let start = range.first()?.span;
        let end = range.last()?.span;
        for tok in range {
            tok.is_known = true;
        }
        Some(Segment {
            kind: SegmentKind::EpisodeTitle,
            text: Cow::Owned(value),
            pos: Range::from(start.start..end.end),
        })
    }
}

fn find_episode_title<'t, 's>(tokens: &'t mut [Token<'s>]) -> Option<&'t mut [Token<'s>]> {
    let mut first = tokens.iter().position(|t| !t.is_enclosed && t.is_free() && !t.is_known);
    let mut last =
        first.and_then(|idx| find_next_token(tokens, idx, false, |t| t.is_known || matches!(t.kind, TokenKind::OpenPunc(_))));

    if first.is_none() {
        first = tokens.iter()
            .position(|t| matches!(t.kind, TokenKind::OpenPunc("「")))
            .map(|i| i + 1);
        last =
            first.and_then(|idx| find_next_token(tokens, idx, false, |t| t.is_known || matches!(t.kind, TokenKind::ClosePunc("」"))));
        let l = last?;
        if tokens[first.unwrap_or_default()..l].iter().any(|t| t.is_known) {
            return None;
        }
    }

    match (first, last) {
        (Some(x), Some(y)) => Some(&mut tokens[x..y]),
        (Some(x), None) => Some(&mut tokens[x..]),
        _ => None
    }

}

// @todo: remove all (most) string allocation (the regex items already return cow strings, so start there) <- preallocated slab??
// clean title for use in data source pinging
fn process_title(title: &str, parts: &[Segment<'_>]) -> String {
    // we'll be draining this
    let mut title = title.to_string();

    // @todo: actually try to parse + categorize website
    // ^^ this must be done at the same time as searching for keywords
    // strip websites
    title = WEBSITE_TYPE_1_REGEX.replace(&title, "").to_string();
    title = WEBSITE_TYPE_2_REGEX.replace(&title, "").to_string();
    title = WEBSITE_TYPE_3_REGEX.replace(&title, "").to_string();
    // we explicitly do not try and remove any other instance of non-enclosed non-english characters
    // as they can be part of the real title (ex. `わかっていますよ旦那さま。どうせ「愛する人ができた」と言うんでしょ？～ドアマットヒロイン、頭をぶつけた拍子に前世が大阪のオバチャンだった事を思い出す～ THE COMIC`)
    //
    // this, however, does leave actual alternative title cases like `Boku no Kokoro no Yabai Yatsu 僕の心のヤバイやつ` untouched;
    // but, i will deal with it during the data source pinging stage by using fallback search queries
    //
    // replace all occurrances to rip out duplicates like `[北宇治字幕组] 淡岛百景 / 淡島百景 / Awajima Hyakkei [07][WebRip][HEVC_AAC][简繁日内封]`
    title = ALT_TITLES.replace_all(&title, "").to_string();
    title = ALT_TITLES_ENCLOSED.replace(&title, "$1").to_string();
    title = REM_BRACKETED_ITEMS.replace(&title, "").to_string();

    if let Some(part) = parts.iter().find(|p| matches!(p.kind, SegmentKind::Genre | SegmentKind::Episode)) {
        // strip out the genre for cases like `The Deep Blue Sea - Drama`, and episodes if they already exist
        // iff it exists after a delimeter like `-`
        let delimeter = title.chars().position(|c| ['-'].contains(&c)).unwrap_or(title.len());
        if title.match_indices(&*part.text).all(|(i, _)| i >= delimeter) {
            title = title.trim_end_matches(&*part.text).to_string();
        }
    }
    if let Some(part) = parts.iter()
        .find(|p| matches!(p.kind, SegmentKind::Season) && p.text.eq_ignore_ascii_case("season"))
        && parts.iter().any(|p| matches!(p.kind, SegmentKind::Season) && p.text.chars().all(|c| c.is_ascii_digit()))
    {
        // @todo: also strip any season shorthands (e.g. `S2`) if they appear in the title
        // ex. `[YYQSUB][Youjo Senki S2][02][TVRIP][1080P][AVC_AAC][JPSC].mp4`
        // strip any season keywords from the title if there also exist numeric season entries
        // ex. `Homeland.Season.1-4.Complete.720p.HDTV.X264-MRSK`
        title = title.trim_end_matches(&*part.text).to_string();
    }
    title = title.trim_matches(['-', ' ', '.']).to_string();
    title
}
