use fancy_regex::Regex;
use regex::Regex as PlainRegex;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

pub const WIKI_GROUP: &str = "Signs of AI writing (Wikipedia)";
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
    fn plain(start: usize, end: usize) -> Self {
        Match {
            pattern: "",
            start,
            end,
            count: None,
            note: None,
        }
    }

    fn counted(start: usize, end: usize, count: usize, note: String) -> Self {
        Match {
            pattern: "",
            start,
            end,
            count: Some(count),
            note: Some(note),
        }
    }
}

pub struct Pattern {
    pub id: &'static str,
    pub group: Option<&'static str>,
    pub name: &'static str,
    pub description: &'static str,
    pub hint: &'static str,
    finder: Finder,
}

impl Pattern {
    pub fn find(&self, text: &str) -> Vec<Match> {
        let mut found = self.finder.find(text);
        for m in &mut found {
            m.pattern = self.id;
        }
        found
    }
}

enum Finder {
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
    fn find(&self, text: &str) -> Vec<Match> {
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

fn re(pattern: &str) -> Regex {
    Regex::new(pattern).unwrap_or_else(|e| panic!("bad pattern {pattern}: {e}"))
}

const CHAIN_BODY: &str = r"[^,.;:!?\n\x{2013}\x{2014}\x{2026}]*";
const CHAIN_SEP: &str = r"(?:\s*,\s*(?:and\s+|or\s+)?|\s+(?:and|or)\s+|\s*[;&\x{2013}\x{2014}]\s*(?:and\s+|or\s+)?|\s+-{1,2}\s+)";

static CHAIN_SPLIT: LazyLock<PlainRegex> =
    LazyLock::new(|| PlainRegex::new(&format!("(?i){CHAIN_SEP}")).unwrap());

fn chain(head: &str, head_test: &str, label: &'static str) -> Finder {
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

fn char_before(text: &str, i: usize) -> Option<char> {
    text[..i].chars().next_back()
}

fn char_at(text: &str, i: usize) -> Option<char> {
    text[i..].chars().next()
}

fn trim_end_ws(text: &str, floor: usize, mut end: usize) -> usize {
    while end > floor {
        match char_before(text, end) {
            Some(c) if c.is_whitespace() => end -= c.len_utf8(),
            _ => break,
        }
    }
    end
}

fn trim_start_ws(text: &str, mut start: usize, ceil: usize) -> usize {
    while start < ceil {
        match char_at(text, start) {
            Some(c) if c.is_whitespace() => start += c.len_utf8(),
            _ => break,
        }
    }
    start
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

pub static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(build_patterns);

fn build_patterns() -> Vec<Pattern> {
    let p = |id, name, description, finder| Pattern {
        id,
        group: None,
        name,
        description,
        hint: hint_for(id),
        finder,
    };
    let w = |id, name, description, finder| Pattern {
        id,
        group: Some(WIKI_GROUP),
        name,
        description,
        hint: hint_for(id),
        finder,
    };
    let rx = |s: &str| Finder::Regex(re(s));
    vec![
        p(
            "no-chain",
            "\u{201c}No X, no Y\u{201d} chains",
            "Two or more \u{201c}no \u{2026}\u{201d} items in a row, e.g. \u{201c}No fluff, no filler, no jargon.\u{201d} The count is the number of \u{201c}no\u{201d} items.",
            chain(r"no[-\s]", r"^no[-\s]", "\u{201c}no\u{201d} item"),
        ),
        p(
            "whole",
            "\u{201c}That\u{2019}s the whole \u{2026}\u{201d}",
            "\u{201c}That / this is the whole point, game, thing \u{2026}\u{201d}",
            rx(r"(?i)\b(?:that|this)(?:['\x{2019}]s|\s+(?:is|was))\s+the\s+whole\b(?:\s+\w+)?"),
        ),
        p(
            "did-not-chain",
            "\u{201c}Did not X, did not Y\u{201d} chains",
            "Two or more \u{201c}did not \u{2026}\u{201d} or \u{201c}didn\u{2019}t \u{2026}\u{201d} items in a row. The count is the number of items.",
            chain(
                r"(?:did\s+not|didn['\x{2019}]t)\s",
                r"^(?:did\s+not|didn['\x{2019}]t)\s",
                "\u{201c}did not\u{201d} item",
            ),
        ),
        p(
            "dont-verb-it",
            "\u{201c}Don\u{2019}t VERB it \u{2026} VERB it\u{201d}",
            "\u{201c}Don\u{2019}t call it X. Call it Y.\u{201d}: a negated verb + \u{201c}it\u{201d}, then the same verb + \u{201c}it\u{201d} again.",
            rx(
                r#"(?i)\b(?:do\s+not|don['\x{2019}]t)\s+(?:just\s+|simply\s+|merely\s+)?(\w+)(?:\s+(?:of|about|at|on|for|with|to))?\s+it\b[^.!?\n]*?[.!?;,:\x{2013}\x{2014}]['\x{201d}\x{2019}"]*\s*(?:just\s+|simply\s+|merely\s+)?\1(?:\s+(?:of|about|at|on|for|with|to))?\s+it\b"#,
            ),
        ),
        p(
            "sit-with",
            "\u{201c}Sit with that\u{201d}",
            "The reflective \u{201c}sit with that / this / it (for a moment)\u{201d}, plus \u{201c}sit with the discomfort\u{201d} and friends.",
            rx(
                r"(?i)\bsit(?:s|ting)?\s+with\s+(?:that|this|it|(?:the|your)\s+(?:discomfort|feelings?|tension|weight|uncertainty|ambiguity|grief|silence|unease))\b(?:\s+for\s+a\s+\w+)?",
            ),
        ),
        p(
            "already-know",
            "\u{201c}You already know\u{201d}",
            "\u{201c}You already know\u{201d}: the answer, what to do, or standing alone before a full stop.",
            rx(
                r"(?i)\byou\s+already\s+knows?\s+(?:the\s+answer|what|how|why|this|that|it|who|where)\b|\byou\s+already\s+knows?\b(?![ \t]+\w)",
            ),
        ),
        p(
            "is-the-entire",
            "\u{201c}Is the entire \u{2026}\u{201d}",
            "\u{201c}X is the entire point / game / business model.\u{201d}",
            rx(r"(?i)(?:\b(?:is|was|are|were)|['\x{2019}]s)\s+the\s+entire\b(?:\s+\w+)?"),
        ),
        p(
            "the-entire-is",
            "\u{201c}The entire \u{2026} is\u{201d}",
            "\u{201c}The entire point / game / business model is \u{2026}\u{201d}, the flipped twin of \u{201c}is the entire\u{201d}.",
            rx(
                r"(?i)\bthe\s+entire\s+[\w'\x{2019}-]+(?:\s+[\w'\x{2019}-]+){0,4}?\s+(?:is|was|are|were)\b",
            ),
        ),
        p(
            "is-real",
            "\u{201c}Is real \u{2026} and / not\u{201d}",
            "\u{201c}The X is real, and / not \u{2026}\u{201d}, including \u{201c}is the real \u{2026} and it\u{201d}. Skips \u{201c}real estate\u{201d}, \u{201c}real time\u{201d}, and similar.",
            rx(
                r"(?i)\bis\s+(?:(?:the|a)\s+real\b(?![\s-]+(?:estate|time|life|world|quick)\b)[^.!?\n]*?\b(?:and|not)\s+it\b|real\b(?![\s-]+(?:estate|time|life|world|quick)\b)[^.!?\n]*?\b(?:and|not)\b)",
            ),
        ),
        p(
            "punchline",
            "\u{201c}The punchline is\u{201d}",
            "\u{201c}The punchline is \u{2026}\u{201d}, \u{201c}the punchline:\u{201d}, or \u{201c}the punchline?\u{201d}.",
            rx(r"(?i)\bthe\s+punchline(?:\s+(?:is|was|being)\b|\s*[:?])"),
        ),
        p(
            "worth-naming",
            "\u{201c}Worth naming\u{201d}",
            "The therapist-voiced \u{201c}that loss is real and it\u{2019}s worth naming\u{201d}, \u{201c}it\u{2019}s worth naming that \u{2026}\u{201d}, or a \u{201c}Worth naming:\u{201d} opener. Skips \u{201c}naming names\u{201d}.",
            rx(
                r"(?i)(?:\b(?:is|are|was|were|feels?|felt|seems?|seemed)|['\x{2019}]s)\s+(?:\w+\s+){0,2}?worth\s+naming\b(?!\s+names\b)|\bworth\s+naming\s*:",
            ),
        ),
        p(
            "not-nothing",
            "\u{201c}That\u{2019}s not nothing\u{201d}",
            "\u{201c}That is not nothing\u{201d} / \u{201c}that\u{2019}s not nothing\u{201d}, plus the \u{201c}this / it / which is not nothing\u{201d} variants.",
            rx(r"(?i)\b(?:that|this|it|which)(?:['\x{2019}]s|\s+(?:is|was))\s+not\s+nothing\b"),
        ),
        p(
            "is-the-whole",
            "\u{201c}Is the whole \u{2026}\u{201d}",
            "Any subject + \u{201c}is the whole point / trick / pitch / idea\u{201d}, plus the \u{201c}here is the whole \u{2026}\u{201d} opener.",
            rx(
                r"(?i)(?:\b(?:is|was|are|were)|['\x{2019}]s)\s+the\s+whole\b(?:\s+\w+)?|\bhere(?:['\x{2019}]s|\s+is)\s+the\s+whole\b(?:\s+\w+)?",
            ),
        ),
        p(
            "echo-triad",
            "Echoing sentence runs",
            "Consecutive sentences built on the same repeated skeleton: \u{201c}A shopping cart is an object in the system. A chat room is an object in the system.\u{201d} The count is the number of echoing sentences.",
            Finder::Echo {
                min_gram: 4,
                min_run: 2,
            },
        ),
        p(
            "performative-honesty",
            "Performative honesty",
            "Sincerity announced rather than demonstrated: \u{201c}I won\u{2019}t pretend\u{201d}, \u{201c}I\u{2019}ll be honest\u{201d}, \u{201c}let\u{2019}s be honest\u{201d}, \u{201c}to be clear\u{201d}, and sentence-initial \u{201c}Honestly,\u{201d} or \u{201c}Look,\u{201d}.",
            rx(
                r"(?i)\bI\s+(?:will\s+not|won['\x{2019}]t)\s+pretend\b|\b(?:I['\x{2019}]ll|let['\x{2019}]s|to)\s+be\s+(?:honest|clear|blunt|real)\b|(?:^|[.!?\x{2013}\x{2014}]\s+|\n)(?:Honestly|Look|Truthfully|Frankly)\s*,",
            ),
        ),
        p(
            "thats-the-part",
            "\u{201c}That\u{2019}s the part \u{2026}\u{201d}",
            "Gesturing at a favoured detail instead of stating it: \u{201c}that is the part a counter can\u{2019}t reach\u{201d}, \u{201c}the part that makes me trust the rest\u{201d}, \u{201c}my favourite part of \u{2026}\u{201d}.",
            rx(
                r"(?i)\b(?:that|this|it)(?:['\x{2019}]s|\s+(?:is|was))\s+the\s+part\b|\bthe\s+part\s+that\s+(?:makes|made|gets|got|keeps|kept)\s+(?:me|you|us|it)\b|\bmy\s+favou?rite\s+part\s+of\b",
            ),
        ),
        p(
            "the-only-i-trust",
            "\u{201c}The only X I trust\u{201d}",
            "The narrowing superlative reveal: \u{201c}the only marketing I trust\u{201d}, \u{201c}the only thing it needs\u{201d}, \u{201c}the only X that matters\u{201d}.",
            rx(
                r"(?i)\bthe\s+only\s+[\w'\x{2019}-]+(?:\s+[\w'\x{2019}-]+){0,2}?\s+(?:I|you|we|it|he|she|they)\s+(?:trust|need|needs|care|want|wants|use|uses|believe)\b|\bthe\s+only\s+[\w'\x{2019}-]+\s+that\s+(?:matters|counts|works|survives)\b",
            ),
        ),
        p(
            "take-my-word",
            "\u{201c}Don\u{2019}t take my word for it\u{201d}",
            "The stock invitation to verify: \u{201c}you don\u{2019}t have to take my word for it\u{201d}, \u{201c}don\u{2019}t take my word for any of this\u{201d}.",
            rx(
                r"(?i)\b(?:you\s+)?(?:do\s+not|don['\x{2019}]t)\s+(?:have\s+to\s+)?take\s+my\s+word\s+for\s+(?:it|any\s+of\s+(?:it|this|that))\b",
            ),
        ),
        p(
            "turns-out",
            "\u{201c}Turns out \u{2026}\u{201d}",
            "The casual-revelation opener, almost always bolted to a tidy conclusion: \u{201c}Turns out X\u{201d}, \u{201c}it turns out that X\u{201d}.",
            rx(r"(?i)(?:^|[.!?\x{2013}\x{2014}]\s+|\n)Turns\s+out\b|\bit\s+turns\s+out\s+that\b"),
        ),
        p(
            "fits-in-your-head",
            "\u{201c}Fits in your head\u{201d}",
            "Dev-blog boilerplate for simplicity: \u{201c}small enough to hold in your head\u{201d}, \u{201c}batteries included\u{201d}, \u{201c}it just works\u{201d}, \u{201c}zero config\u{201d}, \u{201c}sane defaults\u{201d}.",
            rx(
                r"(?i)\b(?:hold|fit|fits|holds|held)\s+(?:it\s+)?in\s+your\s+head\b|\bbatteries[-\s]included\b|\bit\s+just\s+works\b|\bzero[-\s]config(?:uration)?\b|\bsane\s+defaults\b",
            ),
        ),
        p(
            "stacked-questions",
            "Stacked rhetorical questions",
            "Two or more questions fired in a row, usually fragments after the first: \u{201c}Do I know how it works? Where it breaks? Which corners it cut?\u{201d} The count is the number of questions.",
            Finder::Questions { min_run: 2 },
        ),
        p(
            "sentence-anaphora",
            "Repeated sentence openers",
            "Three or more consecutive sentences starting on the same word: \u{201c}Maybe nobody needed it. Maybe it introduced \u{2026} Maybe a small convenience \u{2026}\u{201d} Pronouns and articles are ignored. The count is the number of sentences.",
            Finder::Anaphora { min_run: 3 },
        ),
        p(
            "colon-triple",
            "Colon into a triple",
            "A colon opening onto three or more comma-separated items: \u{201c}separate ports, processes, and local state\u{201d}. Noisy in technical writing, consider --skip colon-triple for documentation.",
            rx(
                r":\s+[^.!?;:\n]{2,40},\s+[^.!?;:\n]{2,40},\s+(?:and\s+|or\s+)?[^.!?;:\n]{2,40}(?=[.!?\n])",
            ),
        ),
        p(
            "heres-the-twist",
            "\u{201c}Here\u{2019}s the twist\u{201d}",
            "The stage-managed reveal: \u{201c}here\u{2019}s the twist\u{201d}, \u{201c}here\u{2019}s the thing\u{201d}, \u{201c}here\u{2019}s the catch / kicker / rub\u{201d}, \u{201c}here\u{2019}s the first example:\u{201d}.",
            rx(
                r"(?i)\bhere(?:['\x{2019}]s|\s+is)\s+(?:the|a|my|one)\s+(?:twist|thing|catch|kicker|rub|problem|first|second|third|next|recent|real|best|worst|surprising|interesting|key|important)\b[\w\s-]{0,20}[:.]",
            ),
        ),
        p(
            "x-is-dead",
            "\u{201c}X is dead\u{201d}",
            "The obituary headline and its sequel: \u{201c}peer code review is dead\u{201d}, \u{201c}botd is dead; long live botd\u{201d}.",
            rx(r"(?i)\b[\w\s]{3,30}\s+(?:is|are)\s+dead\b|\blong\s+live\s+\w+"),
        ),
        p(
            "thats-why-mattered",
            "\u{201c}That\u{2019}s why X mattered\u{201d}",
            "Retroactively assigning significance: \u{201c}that\u{2019}s why being able to open the environment mattered\u{201d}, \u{201c}this is why preserving every conversation mattered\u{201d}.",
            rx(
                r"(?i)\b(?:that|this)(?:['\x{2019}]s|\s+(?:is|was))\s+why\b[^.!?\n]{0,80}?\b(?:matter(?:s|ed)?|count(?:s|ed)?)\b",
            ),
        ),
        p(
            "stranded-auxiliary",
            "Stranded auxiliary contrast",
            "A clause that lands on a bare auxiliary for the reversal: \u{201c}The tool died; the data didn\u{2019}t.\u{201d}, \u{201c}Reading mostly passed \u{2026} Writing didn\u{2019}t\u{201d}, \u{201c}Maybe it wouldn\u{2019}t have.\u{201d}",
            rx(
                r"[;:,]\s+[^.;:!?\n]{2,50}\s(?:did|does|do|was|were|is|are|has|have|had|can|could|would|will)(?:n['\x{2019}]t)?\s*[.;]|\b(?:Maybe|Perhaps)\s+\w+[^.!?\n]{0,40}\s(?:would|could|might|should|did|had|was|is)(?:n['\x{2019}]t)?\s+(?:have\s*)?\.",
            ),
        ),
        w(
            "ai-vocab",
            "AI vocabulary words",
            "Words LLMs lean on far more than people do: \u{201c}delve\u{201d}, \u{201c}tapestry\u{201d}, \u{201c}meticulous\u{201d}, \u{201c}pivotal\u{201d}, \u{201c}intricate\u{201d}, \u{201c}interplay\u{201d}, \u{201c}underscore\u{201d}, \u{201c}garner\u{201d}, \u{201c}bolster\u{201d}, \u{201c}vibrant\u{201d}, \u{201c}bustling\u{201d}, \u{201c}multifaceted\u{201d}, \u{201c}seamless\u{201d}, \u{201c}ever-evolving\u{201d}. One hit can be coincidence, several is a tell.",
            rx(
                r"(?i)\b(?:delv(?:e|es|ed|ing)|tapestr(?:y|ies)|meticulous(?:ly)?|pivotal|intricate(?:ly)?|intricacies|interplay|underscor(?:e|es|ed|ing)|garner(?:s|ed|ing)?|bolster(?:s|ed|ing)?|vibrant|bustling|multifaceted|seamless(?:ly)?|commendable|ever-evolving)\b",
            ),
        ),
        w(
            "not-just",
            "\u{201c}Not just X, but Y\u{201d}",
            "Negative parallelisms: \u{201c}not just X, but (also) Y\u{201d}, \u{201c}not only \u{2026} but \u{2026}\u{201d}, and the \u{201c}it\u{2019}s not X, it\u{2019}s Y\u{201d} contrast.",
            rx(
                r"(?i)\bnot\s+(?:just|only|merely|simply)\s+[^.!?\n;]*?\bbut(?:\s+also)?\b|\b(?:it|this|that)(?:['\x{2019}]s|\s+(?:is|was))\s+not\s+[^.!?\n,;\x{2014}\x{2013}]{1,60}[,;\x{2014}\x{2013}]\s*(?:it|this|that)(?:['\x{2019}]s|\s+(?:is|was))\b",
            ),
        ),
        w(
            "note-that",
            "\u{201c}It\u{2019}s important to note\u{201d}",
            "Didactic hedging: \u{201c}it is important to note that\u{201d}, \u{201c}it\u{2019}s worth noting\u{201d}, \u{201c}it should be noted\u{201d}, plus the \u{201c}worth pausing / considering / asking\u{201d} family.",
            rx(
                r"(?i)\bit(?:['\x{2019}]s|\s+(?:is|was))\s+(?:also\s+)?(?:important|worth|crucial|essential|vital)\s+(?:to\s+(?:note|remember|understand|recognize|mention|pause|consider|ask)|noting|mentioning|remembering|pausing|considering|asking)\b(?:\s+that\b)?|\bit\s+should\s+be\s+noted\b",
            ),
        ),
        w(
            "testament",
            "\u{201c}Stands as a testament\u{201d}",
            "\u{201c}Stands / serves as a testament (or reminder)\u{201d}, \u{201c}is a testament to\u{201d}: inflating significance instead of saying what happened.",
            rx(
                r"(?i)\b(?:stand|stands|stood|serve|serves|served|standing|serving)\s+as\s+(?:a|an)\s+(?:\w+\s+)?(?:testament|reminder)\b|\b(?:is|was|are|were|remain|remains)\s+a\s+(?:\w+\s+)?testament\s+to\b",
            ),
        ),
        w(
            "crucial-role",
            "\u{201c}Plays a crucial role\u{201d}",
            "\u{201c}Plays a crucial / pivotal / vital / key / significant role in \u{2026}\u{201d}.",
            rx(
                r"(?i)\bplay(?:s|ed|ing)?\s+(?:a|an)\s+(?:\w+\s+)?(?:crucial|pivotal|vital|key|significant|central|critical|important)\s+role\b",
            ),
        ),
        w(
            "landscape",
            "\u{201c}Ever-evolving landscape\u{201d}",
            "Scene-setting boilerplate: \u{201c}the ever-evolving / changing / shifting landscape\u{201d}, \u{201c}in today\u{2019}s fast-paced world\u{201d}.",
            rx(
                r"(?i)\b(?:ever-)?(?:evolving|changing|shifting)\s+landscape\b|\bin\s+today['\x{2019}]s\s+(?:fast-paced|ever-changing|ever-evolving|digital|modern|competitive)\s+\w+",
            ),
        ),
        w(
            "vague-experts",
            "\u{201c}Experts argue\u{201d}",
            "Vague attribution to unnamed authorities: \u{201c}experts argue\u{201d}, \u{201c}some critics have noted\u{201d}, \u{201c}observers suggest\u{201d}, \u{201c}industry reports indicate\u{201d}.",
            rx(
                r"(?i)\b(?:many|some|several|most|numerous)?\s*(?:experts|critics|observers|scholars|analysts|commentators)\s+(?:have\s+|often\s+|widely\s+)?(?:argu(?:e|es|ed)|not(?:e|es|ed)|suggest(?:s|ed)?|believ(?:e|es|ed)|agree[ds]?|contend(?:s|ed)?|observ(?:e|es|ed)|caution(?:s|ed)?|claim(?:s|ed)?|cit(?:e|es|ed)|point(?:s|ed)?\s+out)\b|\bindustry\s+reports?\s+(?:suggest|indicate|show)\w*\b",
            ),
        ),
        w(
            "despite-challenges",
            "\u{201c}Despite these challenges\u{201d}",
            "The boilerplate challenges-and-outlook formula: \u{201c}despite these challenges\u{201d}, \u{201c}faces several challenges\u{201d}, \u{201c}challenges remain\u{201d}, \u{201c}remains to be seen\u{201d}, \u{201c}time will tell\u{201d}.",
            rx(
                r"(?i)\bdespite\s+(?:these|those|such|its|their|the|numerous|significant|ongoing)\s+(?:\w+\s+)?challenges\b|\bfac(?:e|es|ed|ing)\s+(?:several|numerous|many|significant|various|a\s+number\s+of)\s+challenges\b|\bchallenges\s+remain\b|\bremains\s+to\s+be\s+seen\b|\b(?:only\s+)?time\s+will\s+tell\b",
            ),
        ),
        w(
            "participle-tail",
            "Participle sentence tails",
            "Superficial analysis bolted onto a sentence end: \u{201c}\u{2026}, highlighting / underscoring / showcasing / reflecting the \u{2026}\u{201d}.",
            rx(
                r"(?i),\s+(?:highlighting|underscoring|emphasizing|showcasing|reflecting|demonstrating|illustrating|signaling|solidifying|cementing|reinforcing|underlining)\s+(?:its|his|her|their|our|the|a|an|how|that|what|both)\b[^.!?\n]*",
            ),
        ),
        w(
            "promo",
            "Promotional boilerplate",
            "Travel-brochure tone: \u{201c}nestled in\u{201d}, \u{201c}in the heart of\u{201d}, \u{201c}rich tapestry / heritage\u{201d}, \u{201c}hidden gem\u{201d}, \u{201c}boasts a\u{201d}, \u{201c}breathtaking\u{201d}, \u{201c}stunning views\u{201d}.",
            rx(
                r"(?i)\bnestled\s+(?:in|on|among|between|along|at)\b|\bin\s+the\s+heart\s+of\b|\brich\s+(?:cultural\s+|historical\s+)?(?:heritage|history|tapestry)\b|\bhidden\s+gem\b|\bmust-(?:visit|see|try)\b|\bbreathtaking\b|\bboasts?\s+(?:a|an|the)\b|\bstunning\s+(?:views?|scenery|architecture|backdrop)\b",
            ),
        ),
        w(
            "ai-leftovers",
            "Chatbot leftovers",
            "Artifacts pasted straight from a chatbot: \u{201c}as an AI language model\u{201d}, \u{201c}as of my last update\u{201d}, \u{201c}knowledge cutoff\u{201d}, plus markup debris like \u{201c}oaicite\u{201d}, \u{201c}contentReference\u{201d}, \u{201c}turn0search\u{201d} and \u{201c}utm_source=\u{201d} tracking parameters.",
            rx(
                r"(?i)\bas\s+an\s+ai(?:\s+language)?\s+model\b|\bas\s+of\s+my\s+last\s+(?:update|training)\b|\bknowledge\s+cutoff\b|\bI\s+(?:cannot|can['\x{2019}]t|do\s+not|don['\x{2019}]t)\s+(?:browse\s+the\s+internet|access\s+real-?time)\b|contentReference|oaicite|turn0(?:search|news|image)\d*|attributableIndex|utm_source=",
            ),
        ),
    ]
}

fn hint_for(id: &str) -> &'static str {
    match id {
        "no-chain" => "Keep one \u{201c}no\u{201d} item or rewrite as a plain positive statement of what it is.",
        "whole" => "State the point directly instead of announcing that it is the whole point.",
        "did-not-chain" => "Collapse to one clause, or say what did happen instead of listing what did not.",
        "dont-verb-it" => "Drop the negated half and just use the preferred word.",
        "sit-with" => "Delete the sentence, or say what the reader should conclude.",
        "already-know" => "Say the thing instead of claiming the reader already knows it.",
        "is-the-entire" => "Replace \u{201c}the entire X\u{201d} with the concrete claim.",
        "the-entire-is" => "Replace \u{201c}the entire X\u{201d} with the concrete claim.",
        "is-real" => "Name the specific problem instead of asserting it is real.",
        "punchline" => "Delete the announcement and state the conclusion.",
        "worth-naming" => "Delete \u{201c}worth naming\u{201d} and name it.",
        "not-nothing" => "Say how much it matters, with a number or a concrete consequence.",
        "is-the-whole" => "State the point directly instead of announcing that it is the whole point.",
        "echo-triad" => "Vary the sentence structure or merge the echoing sentences into one.",
        "performative-honesty" => "Delete the sincerity marker and keep the claim.",
        "thats-the-part" => "State the detail instead of gesturing at it.",
        "the-only-i-trust" => "Replace the superlative with the actual reason.",
        "take-my-word" => "Delete it and give the evidence.",
        "turns-out" => "Delete \u{201c}turns out\u{201d} and state the finding.",
        "fits-in-your-head" => "Say what is small or simple about it, concretely.",
        "stacked-questions" => "Answer the first question and delete the rest.",
        "sentence-anaphora" => "Vary the openers or merge the sentences.",
        "colon-triple" => "Use a sentence, or a bulleted list if the items matter. Skip with --skip colon-triple in technical docs.",
        "heres-the-twist" => "Delete the announcement and state the point.",
        "x-is-dead" => "Say what changed and for whom.",
        "thats-why-mattered" => "State the consequence directly instead of explaining why it mattered.",
        "stranded-auxiliary" => "Finish the clause with the verb and object, or merge it into the previous sentence.",
        "ai-vocab" => "Replace with a plainer word (look into, detailed, careful, key, complex, smooth).",
        "not-just" => "Keep only the second half of the contrast.",
        "note-that" => "Delete the hedge and state the fact.",
        "testament" => "Say what happened instead of what it is a testament to.",
        "crucial-role" => "Say what it does instead of calling its role crucial.",
        "landscape" => "Delete the scene-setting and start with the specific subject.",
        "vague-experts" => "Name the source or drop the attribution.",
        "despite-challenges" => "Name the specific challenge or cut the sentence.",
        "participle-tail" => "End the sentence before the comma, or make the tail its own sentence with a subject.",
        "promo" => "Replace with a factual description.",
        "ai-leftovers" => "Delete the artifact.",
        _ => panic!("no hint for pattern {id}"),
    }
}

pub fn pattern(id: &str) -> Option<&'static Pattern> {
    PATTERNS.iter().find(|p| p.id == id)
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

pub const EXAMPLE: &str = include_str!("example.txt");

#[cfg(test)]
mod tests {
    use super::*;

    fn all() -> HashSet<&'static str> {
        PATTERNS.iter().map(|p| p.id).collect()
    }

    fn only(id: &'static str) -> HashSet<&'static str> {
        HashSet::from([id])
    }

    #[test]
    fn pattern_cases() {
        let cases: &[(&str, &str, usize, Option<&[usize]>)] = &[
            (
                "no-chain",
                "No sign-ups, no downloads, no hassle \u{2014} just paste and go.",
                1,
                Some(&[3]),
            ),
            (
                "no-chain",
                "The plan has no hidden fees and no long-term contracts.",
                1,
                Some(&[2]),
            ),
            (
                "no-chain",
                "No fluff, no filler, no jargon, no corporate buzzwords.",
                1,
                Some(&[4]),
            ),
            (
                "no-chain",
                "There is no catch here, honestly.",
                0,
                Some(&[]),
            ),
            (
                "no-chain",
                "It ships with no bells and whistles, no fluff.",
                1,
                Some(&[2]),
            ),
            ("no-chain", "No, no, I insist.", 0, Some(&[])),
            ("no-chain", "no no no", 0, Some(&[])),
            (
                "no-chain",
                "with no list patterns at all, so nothing lights up.",
                0,
                Some(&[]),
            ),
            (
                "no-chain",
                "NO FEES, NO CONTRACTS, NO SURPRISES",
                1,
                Some(&[3]),
            ),
            ("no-chain", "no fluff; no filler", 1, Some(&[2])),
            (
                "no-chain",
                "no time, no money, no way to say no thanks",
                1,
                Some(&[3]),
            ),
            ("no-chain", "no-code, no-fuss setup", 1, Some(&[2])),
            ("no-chain", "I know nothing, notice nothing.", 0, Some(&[])),
            (
                "no-chain",
                "No fluff, no filler.\nNo ads here.",
                1,
                Some(&[2]),
            ),
            ("whole", "That's the whole point.", 1, None),
            ("whole", "This is the whole game, really.", 1, None),
            ("whole", "That was the whole pitch.", 1, None),
            ("whole", "The whole team showed up.", 0, None),
            (
                "did-not-chain",
                "Did not flinch, did not blink, did not apologize.",
                1,
                Some(&[3]),
            ),
            (
                "did-not-chain",
                "He didn't call and didn't write.",
                1,
                Some(&[2]),
            ),
            ("did-not-chain", "She did not go.", 0, Some(&[])),
            (
                "did-not-chain",
                "Did not know why, did not care.",
                1,
                Some(&[2]),
            ),
            (
                "dont-verb-it",
                "Don't call it a comeback. Call it a return.",
                1,
                None,
            ),
            (
                "dont-verb-it",
                "Do not think of it as a burden. Think of it as fuel.",
                1,
                None,
            ),
            ("dont-verb-it", "Don't fear it. Name it.", 0, None),
            (
                "dont-verb-it",
                "Don\u{2019}t call it \"luck.\" Call it preparation.",
                1,
                None,
            ),
            (
                "dont-verb-it",
                "Don't just read it \u{2014} read it aloud.",
                1,
                None,
            ),
            ("dont-verb-it", "Don't overthink it.", 0, None),
            ("sit-with", "Sit with that for a moment.", 1, None),
            ("sit-with", "Just sit with it.", 1, None),
            ("sit-with", "She was sitting with the discomfort.", 1, None),
            ("sit-with", "Come sit with us at lunch.", 0, None),
            ("already-know", "You already know the answer.", 1, None),
            ("already-know", "Deep down, you already know.", 1, None),
            (
                "already-know",
                "If you already know Python, skip ahead.",
                0,
                None,
            ),
            ("already-know", "You already know what to do.", 1, None),
            ("already-know", "Part of you already knows it.", 1, None),
            ("is-the-entire", "Consistency is the entire game.", 1, None),
            (
                "is-the-entire",
                "That's the entire business model.",
                1,
                None,
            ),
            ("is-the-entire", "He toured the entire factory.", 0, None),
            (
                "the-entire-is",
                "The entire point is that nobody reads.",
                1,
                None,
            ),
            (
                "the-entire-is",
                "The entire business model is built on churn.",
                1,
                None,
            ),
            (
                "the-entire-is",
                "The entire point of the exercise is repetition.",
                1,
                None,
            ),
            ("the-entire-is", "He ate the entire pizza.", 0, None),
            ("the-entire-is", "The entire team was exhausted.", 1, None),
            (
                "the-entire-is",
                "The entire history of the modern industrial world economy is complex.",
                0,
                None,
            ),
            (
                "is-real",
                "The improvement is real, and it's not subtle.",
                1,
                None,
            ),
            (
                "is-real",
                "This is the real work, and it never ends.",
                1,
                None,
            ),
            ("is-real", "The demand is real and growing.", 1, None),
            (
                "is-real",
                "He is a real estate agent and it shows.",
                0,
                None,
            ),
            ("is-real", "Is it real? And does it matter?", 0, None),
            ("is-real", "The painting is real, but stolen.", 0, None),
            (
                "punchline",
                "The punchline is that nobody laughed.",
                1,
                None,
            ),
            ("punchline", "The punchline: nothing changed.", 1, None),
            ("punchline", "And the punchline? You knew.", 1, None),
            ("punchline", "He forgot the punchline entirely.", 0, None),
            (
                "worth-naming",
                "That loss is real and it's worth naming.",
                1,
                None,
            ),
            (
                "worth-naming",
                "It\u{2019}s worth naming that this hurts.",
                1,
                None,
            ),
            ("worth-naming", "The grief here is worth naming.", 1, None),
            (
                "worth-naming",
                "That anger feels worth naming out loud.",
                1,
                None,
            ),
            (
                "worth-naming",
                "Worth naming: nobody asked for this.",
                1,
                None,
            ),
            ("worth-naming", "It's not worth naming names here.", 0, None),
            (
                "worth-naming",
                "They spent the meeting naming the new mascot.",
                0,
                None,
            ),
            (
                "worth-naming",
                "The naming convention is worth documenting.",
                0,
                None,
            ),
            ("not-nothing", "That's not nothing.", 1, None),
            (
                "not-nothing",
                "Ten sign-ups in a week \u{2014} that is not nothing.",
                1,
                None,
            ),
            (
                "not-nothing",
                "It's not nothing, even if it's not everything.",
                1,
                None,
            ),
            (
                "not-nothing",
                "The launch drew a small crowd, which was not nothing.",
                1,
                None,
            ),
            (
                "not-nothing",
                "She insisted that nothing was wrong.",
                0,
                None,
            ),
            ("not-nothing", "There is nothing left to say.", 0, None),
            ("is-the-whole", "Distribution is the whole game.", 1, None),
            (
                "is-the-whole",
                "Here's the whole pitch in one slide.",
                1,
                None,
            ),
            (
                "is-the-whole",
                "That was the whole point of the meeting.",
                1,
                None,
            ),
            ("is-the-whole", "The whole team showed up.", 0, None),
            (
                "echo-triad",
                "A shopping cart is an object in the system. A chat room is an object in the system.",
                1,
                Some(&[2]),
            ),
            (
                "echo-triad",
                "The parser is a state machine. The renderer is a state machine. The scheduler is a state machine.",
                1,
                Some(&[3]),
            ),
            (
                "echo-triad",
                "The parser is fast today. The renderer is fast today.",
                0,
                Some(&[]),
            ),
            (
                "echo-triad",
                "The parser is fast. The tests are slow.",
                0,
                Some(&[]),
            ),
            (
                "performative-honesty",
                "I won't pretend the migration was painless.",
                1,
                None,
            ),
            (
                "performative-honesty",
                "Let's be honest: nobody reads the docs.",
                1,
                None,
            ),
            (
                "performative-honesty",
                "To be clear, the API is unchanged.",
                1,
                None,
            ),
            ("performative-honesty", "Honestly, it was fine.", 1, None),
            ("performative-honesty", "She answered honestly.", 0, None),
            ("performative-honesty", "Look at the diagram.", 0, None),
            (
                "thats-the-part",
                "That's the part a counter can't reach.",
                1,
                None,
            ),
            (
                "thats-the-part",
                "The part that makes me trust the rest is the errata.",
                1,
                None,
            ),
            (
                "thats-the-part",
                "My favorite part of the demo was the undo.",
                1,
                None,
            ),
            (
                "thats-the-part",
                "He played the part of the villain.",
                0,
                None,
            ),
            (
                "the-only-i-trust",
                "It\u{2019}s the only marketing I trust.",
                1,
                None,
            ),
            (
                "the-only-i-trust",
                "The only benchmark that matters is retention.",
                1,
                None,
            ),
            (
                "the-only-i-trust",
                "The only thing it needs is a cache.",
                1,
                None,
            ),
            (
                "the-only-i-trust",
                "She was the only engineer on call.",
                0,
                None,
            ),
            (
                "take-my-word",
                "You don't have to take my word for it.",
                1,
                None,
            ),
            (
                "take-my-word",
                "Don't take my word for any of this.",
                1,
                None,
            ),
            ("take-my-word", "He kept his word.", 0, None),
            ("turns-out", "Turns out the cache was never warm.", 1, None),
            ("turns-out", "It turns out that nobody tested it.", 1, None),
            ("turns-out", "She turns out solid work every week.", 0, None),
            (
                "fits-in-your-head",
                "The design is small enough to hold in your head.",
                1,
                None,
            ),
            (
                "fits-in-your-head",
                "It ships with sane defaults and zero config.",
                2,
                None,
            ),
            (
                "fits-in-your-head",
                "Install it and it just works.",
                1,
                None,
            ),
            (
                "fits-in-your-head",
                "We choose boring technology on purpose.",
                0,
                None,
            ),
            ("fits-in-your-head", "The helmet fits your head.", 0, None),
            (
                "stacked-questions",
                "Do I know how it works? Where it breaks? Which corners it cut?",
                1,
                Some(&[3]),
            ),
            (
                "stacked-questions",
                "Was it worth it? Would I do it again?",
                1,
                Some(&[2]),
            ),
            (
                "stacked-questions",
                "Did it work? Yes, and then some.",
                0,
                Some(&[]),
            ),
            ("stacked-questions", "What changed?", 0, Some(&[])),
            (
                "sentence-anaphora",
                "Maybe nobody needed it. Maybe the timing was off. Maybe both.",
                1,
                Some(&[3]),
            ),
            (
                "sentence-anaphora",
                "Maybe nobody needed it. Maybe the timing was off.",
                0,
                Some(&[]),
            ),
            (
                "sentence-anaphora",
                "The parser is small. The renderer is small. The scheduler is small.",
                0,
                Some(&[]),
            ),
            (
                "sentence-anaphora",
                "Everything changed. Everything slowed down. Everything cost more.",
                1,
                Some(&[3]),
            ),
            (
                "colon-triple",
                "The fix needs three things: separate ports, separate processes, and separate state.",
                1,
                None,
            ),
            (
                "colon-triple",
                "Each service gets its own everything: ports, processes, local state.",
                1,
                None,
            ),
            (
                "colon-triple",
                "The recipe calls for flour, butter, and sugar.",
                0,
                None,
            ),
            ("colon-triple", "Note: the flag is off by default.", 0, None),
            (
                "heres-the-twist",
                "Here's the twist: nobody clicked it.",
                1,
                None,
            ),
            (
                "heres-the-twist",
                "Here is the thing. The demo was fake.",
                1,
                None,
            ),
            (
                "heres-the-twist",
                "Here's a surprising result: it got faster.",
                1,
                None,
            ),
            ("heres-the-twist", "Here's the door code.", 0, None),
            ("x-is-dead", "Peer code review is dead.", 1, None),
            (
                "x-is-dead",
                "The old importer is dead; long live the importer.",
                2,
                None,
            ),
            ("x-is-dead", "Long live the king.", 1, None),
            ("x-is-dead", "He played dead until the bear left.", 0, None),
            (
                "thats-why-mattered",
                "That's why being able to open the environment mattered.",
                1,
                None,
            ),
            (
                "thats-why-mattered",
                "This is why preserving every conversation mattered.",
                1,
                None,
            ),
            (
                "thats-why-mattered",
                "That's why the deadline counts.",
                1,
                None,
            ),
            ("thats-why-mattered", "That is why we left early.", 0, None),
            (
                "stranded-auxiliary",
                "The tool died; the data didn't.",
                1,
                None,
            ),
            (
                "stranded-auxiliary",
                "Reading mostly passed, writing didn't.",
                1,
                None,
            ),
            ("stranded-auxiliary", "Maybe it wouldn't have.", 1, None),
            (
                "stranded-auxiliary",
                "The test passed and the build was green.",
                0,
                None,
            ),
            (
                "ai-vocab",
                "We delve into the intricacies of the interplay.",
                3,
                None,
            ),
            (
                "ai-vocab",
                "Her vibrant tapestry hung in the bustling hall.",
                3,
                None,
            ),
            (
                "ai-vocab",
                "A meticulously curated, seamless experience.",
                2,
                None,
            ),
            (
                "ai-vocab",
                "The report was thorough and well organized.",
                0,
                None,
            ),
            (
                "not-just",
                "This is not just a tool, but a philosophy.",
                1,
                None,
            ),
            ("not-just", "Not only fast but also reliable.", 1, None),
            (
                "not-just",
                "It\u{2019}s not a bug \u{2014} it\u{2019}s a feature.",
                1,
                None,
            ),
            ("not-just", "He did not buy it.", 0, None),
            ("not-just", "She was not sure about the plan.", 0, None),
            (
                "note-that",
                "It is important to note that timing matters.",
                1,
                None,
            ),
            (
                "note-that",
                "It\u{2019}s worth noting the fees are separate.",
                1,
                None,
            ),
            (
                "note-that",
                "It should be noted that this changed in 2020.",
                1,
                None,
            ),
            ("note-that", "It's worth pausing on that number.", 1, None),
            ("note-that", "It is worth asking who benefits.", 1, None),
            ("note-that", "Please note the door code.", 0, None),
            (
                "testament",
                "The building stands as a testament to postwar optimism.",
                1,
                None,
            ),
            (
                "testament",
                "Her career is a testament to persistence.",
                1,
                None,
            ),
            (
                "testament",
                "It serves as a stark reminder that nothing lasts.",
                1,
                None,
            ),
            ("testament", "He read from the Old Testament.", 0, None),
            (
                "crucial-role",
                "Volunteers play a crucial role in the program.",
                1,
                None,
            ),
            (
                "crucial-role",
                "She played a truly pivotal role in the merger.",
                1,
                None,
            ),
            ("crucial-role", "He plays the role of the villain.", 0, None),
            (
                "landscape",
                "Adapting to an ever-evolving landscape.",
                1,
                None,
            ),
            (
                "landscape",
                "The rapidly changing landscape of retail.",
                1,
                None,
            ),
            (
                "landscape",
                "In today\u{2019}s fast-paced world, attention is scarce.",
                1,
                None,
            ),
            (
                "landscape",
                "The landscape outside the window was gray.",
                0,
                None,
            ),
            (
                "vague-experts",
                "Experts argue that the policy failed.",
                1,
                None,
            ),
            (
                "vague-experts",
                "Some critics have noted a decline in quality.",
                1,
                None,
            ),
            (
                "vague-experts",
                "Industry reports suggest strong demand.",
                1,
                None,
            ),
            (
                "vague-experts",
                "Dr. Chen argued the opposite in her paper.",
                0,
                None,
            ),
            (
                "despite-challenges",
                "Despite these challenges, growth continued.",
                1,
                None,
            ),
            (
                "despite-challenges",
                "The sector faces several challenges.",
                1,
                None,
            ),
            (
                "despite-challenges",
                "Whether it works remains to be seen.",
                1,
                None,
            ),
            (
                "despite-challenges",
                "Only time will tell whether it sticks.",
                1,
                None,
            ),
            ("despite-challenges", "Time will tell.", 1, None),
            (
                "despite-challenges",
                "He arrived on time and will tell you himself.",
                0,
                None,
            ),
            ("despite-challenges", "The climb was a challenge.", 0, None),
            (
                "participle-tail",
                "The bridge reopened in June, highlighting the city\u{2019}s investment in infrastructure.",
                1,
                None,
            ),
            (
                "participle-tail",
                "Sales doubled, underscoring the strength of the brand.",
                1,
                None,
            ),
            (
                "participle-tail",
                "She kept highlighting passages in yellow.",
                0,
                None,
            ),
            (
                "participle-tail",
                "The team, reflecting on the loss, regrouped.",
                0,
                None,
            ),
            ("promo", "The inn is nestled in a quiet valley.", 1, None),
            (
                "promo",
                "The museum boasts a rich tapestry of exhibits.",
                2,
                None,
            ),
            ("promo", "Located in the heart of downtown.", 1, None),
            ("promo", "A hidden gem with breathtaking views.", 2, None),
            ("promo", "The soup was rich and hearty.", 0, None),
            (
                "ai-leftovers",
                "As of my last update, the API was in beta.",
                1,
                None,
            ),
            (
                "ai-leftovers",
                "As an AI language model, I cannot form opinions.",
                1,
                None,
            ),
            (
                "ai-leftovers",
                "See example.com/page?utm_source=chatgpt.com for details.",
                1,
                None,
            ),
            (
                "ai-leftovers",
                "contentReference[oaicite:0]{index=0}",
                2,
                None,
            ),
            (
                "ai-leftovers",
                "The last update shipped on Tuesday.",
                0,
                None,
            ),
        ];
        let mut failures = Vec::new();
        for (id, sample, expect, items) in cases {
            let found = pattern(id)
                .unwrap_or_else(|| panic!("unknown pattern {id}"))
                .find(sample);
            if found.len() != *expect {
                failures.push(format!(
                    "{id} \u{b7} {sample:?}: expected {expect} matches, got {}",
                    found.len()
                ));
            }
            if let Some(items) = items {
                let counts: Vec<usize> = found.iter().map(|m| m.count.unwrap_or(0)).collect();
                if counts != *items {
                    failures.push(format!(
                        "{id} \u{b7} {sample:?}: expected counts {items:?}, got {counts:?}"
                    ));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn sentence_bounds_isolate_flagged_sentence() {
        let t = "First sentence here. No fluff, no filler. Last one.";
        let m = &pattern("no-chain").unwrap().find(t)[0];
        let (s, e) = sentence_bounds(t, m.start, m.end);
        assert_eq!(&t[s..e], "No fluff, no filler.");
    }

    fn words(prefix: &str, n: usize) -> String {
        (0..n)
            .map(|i| format!("{prefix}{i}"))
            .collect::<Vec<_>>()
            .join(" ")
    }

    #[test]
    fn excerpt_context_is_twelve_words_each_side() {
        let t = format!(
            "{}. No fluff, no filler, just results. {}.",
            words("w", 30),
            words("t", 30)
        );
        let report = analyze(&t, &only("no-chain"));
        let wins = build_windows(&t, &report.regions);
        assert_eq!(wins.len(), 1);
        assert_eq!(count_words(&t[..wins[0].start]), 18);
        assert_eq!(count_words(&t[wins[0].end..]), 18);
    }

    #[test]
    fn nearby_matches_merge_into_one_window() {
        let t = format!(
            "{}. No fluff, no filler. Ok. No ads, no fees. {}.",
            words("w", 30),
            words("t", 30)
        );
        let report = analyze(&t, &only("no-chain"));
        let wins = build_windows(&t, &report.regions);
        assert_eq!(wins.len(), 1);
        assert_eq!(wins[0].regions.len(), 2);
    }

    #[test]
    fn distant_matches_stay_separate() {
        let t = format!("No fluff, no filler. {}. No ads, no fees.", words("m", 60));
        let report = analyze(&t, &only("no-chain"));
        let wins = build_windows(&t, &report.regions);
        assert_eq!(wins.len(), 2);
        assert_eq!(count_words(&t[wins[0].end..wins[1].start]), 36);
    }

    #[test]
    fn example_trips_every_pattern_once() {
        let report = analyze(EXAMPLE, &all());
        assert_eq!(report.matches.len(), PATTERNS.len());
        let distinct: HashSet<&str> = report.matches.iter().map(|m| m.pattern).collect();
        assert_eq!(distinct.len(), PATTERNS.len());
        assert_eq!(report.regions.len(), PATTERNS.len() - 1);
    }

    #[test]
    fn snippet_truncates_on_chars() {
        let long = "\u{e9}".repeat(100);
        assert_eq!(snippet(&long).chars().count(), 88);
        assert_eq!(snippet("  a \n b  "), "a b");
    }

    #[test]
    fn every_pattern_has_a_hint() {
        for p in PATTERNS.iter() {
            assert!(!p.hint.trim().is_empty(), "{} has no hint", p.id);
        }
    }
}
