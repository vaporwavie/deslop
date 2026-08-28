use crate::report::Match;
use crate::text::{trim_end_ws, trim_start_ws};
use fancy_regex::Regex;
use regex::Regex as PlainRegex;
use std::collections::HashSet;
use std::sync::LazyLock;

pub(crate) enum Finder {
    Regex(Regex),
    Chain {
        chain: Regex,
        head: Regex,
        label: &'static str,
    },
    Echo {
        min_gram: usize,
        min_run: usize,
    },
    Questions {
        min_run: usize,
    },
    Anaphora {
        min_run: usize,
    },
}

impl Finder {
    pub(crate) fn find(&self, text: &str) -> Vec<Match> {
        match self {
            Finder::Regex(re) => re
                .find_iter(text)
                .map(|m| {
                    let m = m.expect("regex step limit");
                    Match::plain(m.start(), m.end())
                })
                .collect(),
            Finder::Chain { chain, head, label } => find_chains(text, chain, head, label),
            Finder::Echo { min_gram, min_run } => find_echoes(text, *min_gram, *min_run),
            Finder::Questions { min_run } => find_question_chains(text, *min_run),
            Finder::Anaphora { min_run } => find_anaphora(text, *min_run),
        }
    }
}

pub(crate) fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("bad pattern {pattern}: {e}"))
}

const CHAIN_BODY: &str = r"[^,.;:!?\n\x{2013}\x{2014}\x{2026}]*";
const CHAIN_SEP: &str = r"(?:\s*,\s*(?:and\s+|or\s+)?|\s+(?:and|or)\s+|\s*[;&\x{2013}\x{2014}]\s*(?:and\s+|or\s+)?|\s+-{1,2}\s+)";

static CHAIN_SPLIT: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(&format!("(?i){CHAIN_SEP}")).unwrap());

pub(crate) fn chain(head: &str, head_test: &str, label: &'static str) -> Finder {
    let item = format!("{head}{CHAIN_BODY}");
    Finder::Chain {
        chain: re(&format!(r"(?i)\b{item}(?:{CHAIN_SEP}{item})+")),
        head: re(&format!("(?i){head_test}")),
        label,
    }
}

fn find_chains(text: &str, chain: &Regex, head: &Regex, label: &str) -> Vec<Match> {
    chain
        .find_iter(text)
        .map(|m| {
            let m = m.expect("regex step limit");
            let end = trim_end_ws(text, m.start(), m.end());
            let count = CHAIN_SPLIT
                .split(m.as_str())
                .filter(|p| head.is_match(p.trim()).unwrap_or(false))
                .count();
            let plural = if count == 1 { "" } else { "s" };
            Match::counted(m.start(), end, count, format!("{count} {label}{plural}"))
        })
        .collect()
}

static ECHO_SENT: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(r"[^.!?\n]+[.!?]?").unwrap());
static ECHO_WORD: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(r"[a-z0-9'\x{2019}-]+").unwrap());

fn grams(sentence: &str, n: usize) -> Vec<String> {
    let lower = sentence.to_lowercase();
    let words: Vec<&str> = ECHO_WORD.find_iter(&lower).map(|m| m.as_str()).collect();
    let mut out: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for w in words.windows(n) {
        let g = w.join(" ");
        if seen.insert(g.clone()) {
            out.push(g);
        }
    }
    out
}

fn find_echoes(text: &str, min_gram: usize, min_run: usize) -> Vec<Match> {
    let sents: Vec<(usize, usize, &str)> = ECHO_SENT
        .find_iter(text)
        .filter(|m| m.as_str().split_whitespace().count() >= 4)
        .map(|m| (m.start(), m.end(), m.as_str()))
        .collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < sents.len() {
        let mut j = i;
        let mut shared: Option<String> = None;
        while j + 1 < sents.len() {
            if sents[j + 1].0 - sents[j].1 > 3 {
                break;
            }
            let a = grams(sents[j].2, min_gram);
            let b: HashSet<String> = grams(sents[j + 1].2, min_gram).into_iter().collect();
            let mut longest: Option<&String> = None;
            for g in a.iter().filter(|g| b.contains(*g)) {
                if longest.is_none_or(|l| g.len() > l.len()) {
                    longest = Some(g);
                }
            }
            match longest {
                Some(g) => shared = Some(g.clone()),
                None => break,
            }
            j += 1;
        }
        let run = j - i + 1;
        match shared {
            Some(g) if run >= min_run => {
                let end = trim_end_ws(text, sents[i].0, sents[j].1);
                found.push(Match::counted(
                    sents[i].0,
                    end,
                    run,
                    format!("{run} sentences echoing \u{201c}{g}\u{201d}"),
                ));
                i = j + 1;
            }
            _ => i += 1,
        }
    }
    found
}

static QUESTION_CHAIN: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(r"[^.!?\n]+\?(?:\s+[^.!?\n]+\?)+").unwrap());

fn find_question_chains(text: &str, min_run: usize) -> Vec<Match> {
    QUESTION_CHAIN
        .find_iter(text)
        .filter_map(|m| {
            let count = m.as_str().matches('?').count();
            if count < min_run {
                return None;
            }
            let start = trim_start_ws(text, m.start(), m.end());
            Some(Match::counted(
                start,
                m.end(),
                count,
                format!("{count} questions in a row"),
            ))
        })
        .collect()
}

const ANAPHORA_SKIP: &[&str] = &[
    "i", "it", "the", "a", "an", "this", "that", "we", "you", "they", "he", "she", "there", "but",
    "and", "so", "in", "as", "if", "my", "his", "her", "their", "its", "these", "those", "for",
    "at", "on", "of", "to", "is", "was",
];
static ANAPHORA_SENT: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(r"[^.!?\n]+[.!?]").unwrap());
static ANAPHORA_WORD: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(r"[A-Za-z'\x{2019}-]+").unwrap());

fn find_anaphora(text: &str, min_run: usize) -> Vec<Match> {
    let sents: Vec<(usize, usize, String)> = ANAPHORA_SENT
        .find_iter(text)
        .filter_map(|m| {
            let w = ANAPHORA_WORD.find(m.as_str())?;
            Some((m.start() + w.start(), m.end(), w.as_str().to_lowercase()))
        })
        .collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i < sents.len() {
        let mut j = i;
        while j + 1 < sents.len() && sents[j + 1].2 == sents[i].2 && sents[j + 1].0 - sents[j].1 < 4
        {
            j += 1;
        }
        let run = j - i + 1;
        if run >= min_run && !ANAPHORA_SKIP.contains(&sents[i].2.as_str()) {
            found.push(Match::counted(
                sents[i].0,
                sents[j].1,
                run,
                format!("{run} sentences opening \u{201c}{}\u{201d}", sents[i].2),
            ));
            i = j + 1;
        } else {
            i += 1;
        }
    }
    found
}
