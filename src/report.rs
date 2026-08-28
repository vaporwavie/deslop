use crate::patterns::PATTERNS;
use crate::text::{char_at, char_before, trim_start_ws};
use serde::Serialize;
use std::collections::{HashMap, HashSet};

pub const CONTEXT_WORDS: usize = 12;

#[derive(Debug, Clone, Serialize)]
pub struct Match {
    pub pattern: &'static str,
    pub start: usize,
    pub end: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl Match {
    pub(crate) fn plain(start: usize, end: usize) -> Self {
        Match {
            pattern: "",
            start,
            end,
            count: None,
            note: None,
        }
    }

    pub(crate) fn counted(start: usize, end: usize, count: usize, note: String) -> Self {
        Match {
            pattern: "",
            start,
            end,
            count: Some(count),
            note: Some(note),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Region {
    pub start: usize,
    pub end: usize,
    pub matches: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub start: usize,
    pub end: usize,
    pub regions: Vec<usize>,
}

pub struct Report {
    pub matches: Vec<Match>,
    pub per_pattern: HashMap<&'static str, usize>,
    pub regions: Vec<Region>,
}

impl Report {
    pub fn chain_items(&self) -> usize {
        self.matches.iter().map(|m| m.count.unwrap_or(0)).sum()
    }
}

pub fn collect_matches(
    text: &str,
    enabled: &HashSet<&str>,
) -> (Vec<Match>, HashMap<&'static str, usize>) {
    let mut per_pattern = HashMap::new();
    let mut raw = Vec::new();
    for p in PATTERNS.iter() {
        per_pattern.insert(p.id, 0);
        if !enabled.contains(p.id) {
            continue;
        }
        raw.extend(p.find(text));
    }
    raw.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));
    let mut matches: Vec<Match> = Vec::new();
    for m in raw {
        if matches.last().is_some_and(|last| m.start < last.end) {
            continue;
        }
        *per_pattern.get_mut(m.pattern).unwrap() += 1;
        matches.push(m);
    }
    (matches, per_pattern)
}

pub fn build_regions(text: &str, matches: &[Match]) -> Vec<Region> {
    let mut regions: Vec<Region> = Vec::new();
    for (i, m) in matches.iter().enumerate() {
        let (s, e) = sentence_bounds(text, m.start, m.end);
        match regions.last_mut() {
            Some(last) if s <= last.end => {
                last.end = last.end.max(e);
                last.matches.push(i);
            }
            _ => regions.push(Region {
                start: s,
                end: e,
                matches: vec![i],
            }),
        }
    }
    regions
}

pub fn analyze(text: &str, enabled: &HashSet<&str>) -> Report {
    let (matches, per_pattern) = collect_matches(text, enabled);
    let regions = build_regions(text, &matches);
    Report {
        matches,
        per_pattern,
        regions,
    }
}

fn is_terminator(c: char) -> bool {
    matches!(c, '\n' | '.' | '!' | '?' | '\u{2026}')
}

pub fn sentence_bounds(text: &str, start: usize, end: usize) -> (usize, usize) {
    let mut s = start;
    while let Some(c) = char_before(text, s) {
        if is_terminator(c) {
            break;
        }
        s -= c.len_utf8();
    }
    s = trim_start_ws(text, s, start);
    let mut e = end;
    while let Some(c) = char_at(text, e) {
        if c == '\n' {
            break;
        }
        e += c.len_utf8();
        if is_terminator(c) {
            while let Some(q) = char_at(text, e) {
                if !matches!(q, '"' | '\'' | '\u{201d}' | '\u{2019}' | ')' | ']') {
                    break;
                }
                e += q.len_utf8();
            }
            break;
        }
    }
    (s, e)
}

pub fn count_words(s: &str) -> usize {
    s.split_whitespace().count()
}

fn expand_left(text: &str, pos: usize, words: usize) -> usize {
    let mut i = pos;
    let mut count = 0;
    while i > 0 && count < words {
        while let Some(c) = char_before(text, i).filter(|c| c.is_whitespace()) {
            i -= c.len_utf8();
        }
        if i == 0 {
            break;
        }
        while let Some(c) = char_before(text, i).filter(|c| !c.is_whitespace()) {
            i -= c.len_utf8();
        }
        count += 1;
    }
    i
}

fn expand_right(text: &str, pos: usize, words: usize) -> usize {
    let mut i = pos;
    let mut count = 0;
    while i < text.len() && count < words {
        while let Some(c) = char_at(text, i).filter(|c| c.is_whitespace()) {
            i += c.len_utf8();
        }
        if i == text.len() {
            break;
        }
        while let Some(c) = char_at(text, i).filter(|c| !c.is_whitespace()) {
            i += c.len_utf8();
        }
        count += 1;
    }
    i
}

pub fn build_windows(text: &str, regions: &[Region]) -> Vec<Window> {
    let mut windows: Vec<Window> = Vec::new();
    for (i, r) in regions.iter().enumerate() {
        let ws = expand_left(text, r.start, CONTEXT_WORDS);
        let we = expand_right(text, r.end, CONTEXT_WORDS);
        match windows.last_mut() {
            Some(last) if ws <= last.end || count_words(&text[last.end..ws]) == 0 => {
                last.end = last.end.max(we);
                last.regions.push(i);
            }
            _ => windows.push(Window {
                start: ws,
                end: we,
                regions: vec![i],
            }),
        }
    }
    windows
}

pub fn snippet(s: &str) -> String {
    let clean = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.chars().count() > 90 {
        let mut out: String = clean.chars().take(87).collect();
        out.push('\u{2026}');
        out
    } else {
        clean
    }
}
