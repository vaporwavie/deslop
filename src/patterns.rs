use crate::finders::{Finder, chain, re};
use crate::report::Match;
use std::sync::LazyLock;

pub const WIKI_GROUP: &str = "Signs of AI writing (Wikipedia)";

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

pub static PATTERNS: LazyLock<Vec<Pattern>> = LazyLock::new(build_patterns);

pub fn pattern(id: &str) -> Option<&'static Pattern> {
    PATTERNS.iter().find(|p| p.id == id)
}

fn build_patterns() -> Vec<Pattern> {
    let p = |id, name, description, hint, finder| Pattern {
        id,
        group: None,
        name,
        description,
        hint,
        finder,
    };
    let w = |id, name, description, hint, finder| Pattern {
        id,
        group: Some(WIKI_GROUP),
        name,
        description,
        hint,
        finder,
    };
    let rx = |s: &str| Finder::Regex(re(s));
    vec![
        p(
            "no-chain",
            "\u{201c}No X, no Y\u{201d} chains",
            "Two or more \u{201c}no \u{2026}\u{201d} items in a row, e.g. \u{201c}No fluff, no filler, no jargon.\u{201d} The count is the number of \u{201c}no\u{201d} items.",
            "Keep one \u{201c}no\u{201d} item or rewrite as a plain positive statement of what it is.",
            chain(r"no[-\s]", r"^no[-\s]", "\u{201c}no\u{201d} item"),
        ),
        p(
            "whole",
            "\u{201c}That\u{2019}s the whole \u{2026}\u{201d}",
            "\u{201c}That / this is the whole point, game, thing \u{2026}\u{201d}",
            "State the point directly instead of announcing that it is the whole point.",
            rx(r"(?i)\b(?:that|this)(?:['\x{2019}]s|\s+(?:is|was))\s+the\s+whole\b(?:\s+\w+)?"),
        ),
        p(
            "did-not-chain",
            "\u{201c}Did not X, did not Y\u{201d} chains",
            "Two or more \u{201c}did not \u{2026}\u{201d} or \u{201c}didn\u{2019}t \u{2026}\u{201d} items in a row. The count is the number of items.",
            "Collapse to one clause, or say what did happen instead of listing what did not.",
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
            "Drop the negated half and just use the preferred word.",
            rx(
                r#"(?i)\b(?:do\s+not|don['\x{2019}]t)\s+(?:just\s+|simply\s+|merely\s+)?(\w+)(?:\s+(?:of|about|at|on|for|with|to))?\s+it\b[^.!?\n]*?[.!?;,:\x{2013}\x{2014}]['\x{201d}\x{2019}"]*\s*(?:just\s+|simply\s+|merely\s+)?\1(?:\s+(?:of|about|at|on|for|with|to))?\s+it\b"#,
            ),
        ),
        p(
            "sit-with",
            "\u{201c}Sit with that\u{201d}",
            "The reflective \u{201c}sit with that / this / it (for a moment)\u{201d}, plus \u{201c}sit with the discomfort\u{201d} and friends.",
            "Delete the sentence, or say what the reader should conclude.",
            rx(
                r"(?i)\bsit(?:s|ting)?\s+with\s+(?:that|this|it|(?:the|your)\s+(?:discomfort|feelings?|tension|weight|uncertainty|ambiguity|grief|silence|unease))\b(?:\s+for\s+a\s+\w+)?",
            ),
        ),
        p(
            "already-know",
            "\u{201c}You already know\u{201d}",
            "\u{201c}You already know\u{201d}: the answer, what to do, or standing alone before a full stop.",
            "Say the thing instead of claiming the reader already knows it.",
            rx(
                r"(?i)\byou\s+already\s+knows?\s+(?:the\s+answer|what|how|why|this|that|it|who|where)\b|\byou\s+already\s+knows?\b(?![ \t]+\w)",
            ),
        ),
        p(
            "is-the-entire",
            "\u{201c}Is the entire \u{2026}\u{201d}",
            "\u{201c}X is the entire point / game / business model.\u{201d}",
            "Replace \u{201c}the entire X\u{201d} with the concrete claim.",
            rx(r"(?i)(?:\b(?:is|was|are|were)|['\x{2019}]s)\s+the\s+entire\b(?:\s+\w+)?"),
        ),
        p(
            "the-entire-is",
            "\u{201c}The entire \u{2026} is\u{201d}",
            "\u{201c}The entire point / game / business model is \u{2026}\u{201d}, the flipped twin of \u{201c}is the entire\u{201d}.",
            "Replace \u{201c}the entire X\u{201d} with the concrete claim.",
            rx(
                r"(?i)\bthe\s+entire\s+[\w'\x{2019}-]+(?:\s+[\w'\x{2019}-]+){0,4}?\s+(?:is|was|are|were)\b",
            ),
        ),
        p(
            "is-real",
            "\u{201c}Is real \u{2026} and / not\u{201d}",
            "\u{201c}The X is real, and / not \u{2026}\u{201d}, including \u{201c}is the real \u{2026} and it\u{201d}. Skips \u{201c}real estate\u{201d}, \u{201c}real time\u{201d}, and similar.",
            "Name the specific problem instead of asserting it is real.",
            rx(
                r"(?i)\bis\s+(?:(?:the|a)\s+real\b(?![\s-]+(?:estate|time|life|world|quick)\b)[^.!?\n]*?\b(?:and|not)\s+it\b|real\b(?![\s-]+(?:estate|time|life|world|quick)\b)[^.!?\n]*?\b(?:and|not)\b)",
            ),
        ),
        p(
            "punchline",
            "\u{201c}The punchline is\u{201d}",
            "\u{201c}The punchline is \u{2026}\u{201d}, \u{201c}the punchline:\u{201d}, or \u{201c}the punchline?\u{201d}.",
            "Delete the announcement and state the conclusion.",
            rx(r"(?i)\bthe\s+punchline(?:\s+(?:is|was|being)\b|\s*[:?])"),
        ),
        p(
            "worth-naming",
            "\u{201c}Worth naming\u{201d}",
            "The therapist-voiced \u{201c}that loss is real and it\u{2019}s worth naming\u{201d}, \u{201c}it\u{2019}s worth naming that \u{2026}\u{201d}, or a \u{201c}Worth naming:\u{201d} opener. Skips \u{201c}naming names\u{201d}.",
            "Delete \u{201c}worth naming\u{201d} and name it.",
            rx(
                r"(?i)(?:\b(?:is|are|was|were|feels?|felt|seems?|seemed)|['\x{2019}]s)\s+(?:\w+\s+){0,2}?worth\s+naming\b(?!\s+names\b)|\bworth\s+naming\s*:",
            ),
        ),
        p(
            "not-nothing",
            "\u{201c}That\u{2019}s not nothing\u{201d}",
            "\u{201c}That is not nothing\u{201d} / \u{201c}that\u{2019}s not nothing\u{201d}, plus the \u{201c}this / it / which is not nothing\u{201d} variants.",
            "Say how much it matters, with a number or a concrete consequence.",
            rx(r"(?i)\b(?:that|this|it|which)(?:['\x{2019}]s|\s+(?:is|was))\s+not\s+nothing\b"),
        ),
        p(
            "is-the-whole",
            "\u{201c}Is the whole \u{2026}\u{201d}",
            "Any subject + \u{201c}is the whole point / trick / pitch / idea\u{201d}, plus the \u{201c}here is the whole \u{2026}\u{201d} opener.",
            "State the point directly instead of announcing that it is the whole point.",
            rx(
                r"(?i)(?:\b(?:is|was|are|were)|['\x{2019}]s)\s+the\s+whole\b(?:\s+\w+)?|\bhere(?:['\x{2019}]s|\s+is)\s+the\s+whole\b(?:\s+\w+)?",
            ),
        ),
        p(
            "echo-triad",
            "Echoing sentence runs",
            "Consecutive sentences built on the same repeated skeleton: \u{201c}A shopping cart is an object in the system. A chat room is an object in the system.\u{201d} The count is the number of echoing sentences.",
            "Vary the sentence structure or merge the echoing sentences into one.",
            Finder::Echo {
                min_gram: 4,
                min_run: 2,
            },
        ),
        p(
            "performative-honesty",
            "Performative honesty",
            "Sincerity announced rather than demonstrated: \u{201c}I won\u{2019}t pretend\u{201d}, \u{201c}I\u{2019}ll be honest\u{201d}, \u{201c}let\u{2019}s be honest\u{201d}, \u{201c}to be clear\u{201d}, and sentence-initial \u{201c}Honestly,\u{201d} or \u{201c}Look,\u{201d}.",
            "Delete the sincerity marker and keep the claim.",
            rx(
                r"(?i)\bI\s+(?:will\s+not|won['\x{2019}]t)\s+pretend\b|\b(?:I['\x{2019}]ll|let['\x{2019}]s|to)\s+be\s+(?:honest|clear|blunt|real)\b|(?:^|[.!?\x{2013}\x{2014}]\s+|\n)(?:Honestly|Look|Truthfully|Frankly)\s*,",
            ),
        ),
        p(
            "thats-the-part",
            "\u{201c}That\u{2019}s the part \u{2026}\u{201d}",
            "Gesturing at a favoured detail instead of stating it: \u{201c}that is the part a counter can\u{2019}t reach\u{201d}, \u{201c}the part that makes me trust the rest\u{201d}, \u{201c}my favourite part of \u{2026}\u{201d}.",
            "State the detail instead of gesturing at it.",
            rx(
                r"(?i)\b(?:that|this|it)(?:['\x{2019}]s|\s+(?:is|was))\s+the\s+part\b|\bthe\s+part\s+that\s+(?:makes|made|gets|got|keeps|kept)\s+(?:me|you|us|it)\b|\bmy\s+favou?rite\s+part\s+of\b",
            ),
        ),
        p(
            "the-only-i-trust",
            "\u{201c}The only X I trust\u{201d}",
            "The narrowing superlative reveal: \u{201c}the only marketing I trust\u{201d}, \u{201c}the only thing it needs\u{201d}, \u{201c}the only X that matters\u{201d}.",
            "Replace the superlative with the actual reason.",
            rx(
                r"(?i)\bthe\s+only\s+[\w'\x{2019}-]+(?:\s+[\w'\x{2019}-]+){0,2}?\s+(?:I|you|we|it|he|she|they)\s+(?:trust|need|needs|care|want|wants|use|uses|believe)\b|\bthe\s+only\s+[\w'\x{2019}-]+\s+that\s+(?:matters|counts|works|survives)\b",
            ),
        ),
        p(
            "take-my-word",
            "\u{201c}Don\u{2019}t take my word for it\u{201d}",
            "The stock invitation to verify: \u{201c}you don\u{2019}t have to take my word for it\u{201d}, \u{201c}don\u{2019}t take my word for any of this\u{201d}.",
            "Delete it and give the evidence.",
            rx(
                r"(?i)\b(?:you\s+)?(?:do\s+not|don['\x{2019}]t)\s+(?:have\s+to\s+)?take\s+my\s+word\s+for\s+(?:it|any\s+of\s+(?:it|this|that))\b",
            ),
        ),
        p(
            "turns-out",
            "\u{201c}Turns out \u{2026}\u{201d}",
            "The casual-revelation opener, almost always bolted to a tidy conclusion: \u{201c}Turns out X\u{201d}, \u{201c}it turns out that X\u{201d}.",
            "Delete \u{201c}turns out\u{201d} and state the finding.",
            rx(r"(?i)(?:^|[.!?\x{2013}\x{2014}]\s+|\n)Turns\s+out\b|\bit\s+turns\s+out\s+that\b"),
        ),
        p(
            "fits-in-your-head",
            "\u{201c}Fits in your head\u{201d}",
            "Dev-blog boilerplate for simplicity: \u{201c}small enough to hold in your head\u{201d}, \u{201c}batteries included\u{201d}, \u{201c}it just works\u{201d}, \u{201c}zero config\u{201d}, \u{201c}sane defaults\u{201d}.",
            "Say what is small or simple about it, concretely.",
            rx(
                r"(?i)\b(?:hold|fit|fits|holds|held)\s+(?:it\s+)?in\s+your\s+head\b|\bbatteries[-\s]included\b|\bit\s+just\s+works\b|\bzero[-\s]config(?:uration)?\b|\bsane\s+defaults\b",
            ),
        ),
        p(
            "stacked-questions",
            "Stacked rhetorical questions",
            "Two or more questions fired in a row, usually fragments after the first: \u{201c}Do I know how it works? Where it breaks? Which corners it cut?\u{201d} The count is the number of questions.",
            "Answer the first question and delete the rest.",
            Finder::Questions { min_run: 2 },
        ),
        p(
            "sentence-anaphora",
            "Repeated sentence openers",
            "Three or more consecutive sentences starting on the same word: \u{201c}Maybe nobody needed it. Maybe it introduced \u{2026} Maybe a small convenience \u{2026}\u{201d} Pronouns and articles are ignored. The count is the number of sentences.",
            "Vary the openers or merge the sentences.",
            Finder::Anaphora { min_run: 3 },
        ),
        p(
            "colon-triple",
            "Colon into a triple",
            "A colon opening onto three or more comma-separated items: \u{201c}separate ports, processes, and local state\u{201d}. Noisy in technical writing, consider --skip colon-triple for documentation.",
            "Use a sentence, or a bulleted list if the items matter. Skip with --skip colon-triple in technical docs.",
            rx(
                r":\s+[^.!?;:\n]{2,40},\s+[^.!?;:\n]{2,40},\s+(?:and\s+|or\s+)?[^.!?;:\n]{2,40}(?=[.!?\n])",
            ),
        ),
        p(
            "heres-the-twist",
            "\u{201c}Here\u{2019}s the twist\u{201d}",
            "The stage-managed reveal: \u{201c}here\u{2019}s the twist\u{201d}, \u{201c}here\u{2019}s the thing\u{201d}, \u{201c}here\u{2019}s the catch / kicker / rub\u{201d}, \u{201c}here\u{2019}s the first example:\u{201d}.",
            "Delete the announcement and state the point.",
            rx(
                r"(?i)\bhere(?:['\x{2019}]s|\s+is)\s+(?:the|a|my|one)\s+(?:twist|thing|catch|kicker|rub|problem|first|second|third|next|recent|real|best|worst|surprising|interesting|key|important)\b[\w\s-]{0,20}[:.]",
            ),
        ),
        p(
            "x-is-dead",
            "\u{201c}X is dead\u{201d}",
            "The obituary headline and its sequel: \u{201c}peer code review is dead\u{201d}, \u{201c}botd is dead; long live botd\u{201d}.",
            "Say what changed and for whom.",
            rx(r"(?i)\b[\w\s]{3,30}\s+(?:is|are)\s+dead\b|\blong\s+live\s+\w+"),
        ),
        p(
            "thats-why-mattered",
            "\u{201c}That\u{2019}s why X mattered\u{201d}",
            "Retroactively assigning significance: \u{201c}that\u{2019}s why being able to open the environment mattered\u{201d}, \u{201c}this is why preserving every conversation mattered\u{201d}.",
            "State the consequence directly instead of explaining why it mattered.",
            rx(
                r"(?i)\b(?:that|this)(?:['\x{2019}]s|\s+(?:is|was))\s+why\b[^.!?\n]{0,80}?\b(?:matter(?:s|ed)?|count(?:s|ed)?)\b",
            ),
        ),
        p(
            "stranded-auxiliary",
            "Stranded auxiliary contrast",
            "A clause that lands on a bare auxiliary for the reversal: \u{201c}The tool died; the data didn\u{2019}t.\u{201d}, \u{201c}Reading mostly passed \u{2026} Writing didn\u{2019}t\u{201d}, \u{201c}Maybe it wouldn\u{2019}t have.\u{201d}",
            "Finish the clause with the verb and object, or merge it into the previous sentence.",
            rx(
                r"[;:,]\s+[^.;:!?\n]{2,50}\s(?:did|does|do|was|were|is|are|has|have|had|can|could|would|will)(?:n['\x{2019}]t)?\s*[.;]|\b(?:Maybe|Perhaps)\s+\w+[^.!?\n]{0,40}\s(?:would|could|might|should|did|had|was|is)(?:n['\x{2019}]t)?\s+(?:have\s*)?\.",
            ),
        ),
        w(
            "ai-vocab",
            "AI vocabulary words",
            "Words LLMs lean on far more than people do: \u{201c}delve\u{201d}, \u{201c}tapestry\u{201d}, \u{201c}meticulous\u{201d}, \u{201c}pivotal\u{201d}, \u{201c}intricate\u{201d}, \u{201c}interplay\u{201d}, \u{201c}underscore\u{201d}, \u{201c}garner\u{201d}, \u{201c}bolster\u{201d}, \u{201c}vibrant\u{201d}, \u{201c}bustling\u{201d}, \u{201c}multifaceted\u{201d}, \u{201c}seamless\u{201d}, \u{201c}ever-evolving\u{201d}. One hit can be coincidence, several is a tell.",
            "Replace with a plainer word (look into, detailed, careful, key, complex, smooth).",
            rx(
                r"(?i)\b(?:delv(?:e|es|ed|ing)|tapestr(?:y|ies)|meticulous(?:ly)?|pivotal|intricate(?:ly)?|intricacies|interplay|underscor(?:e|es|ed|ing)|garner(?:s|ed|ing)?|bolster(?:s|ed|ing)?|vibrant|bustling|multifaceted|seamless(?:ly)?|commendable|ever-evolving)\b",
            ),
        ),
        w(
            "not-just",
            "\u{201c}Not just X, but Y\u{201d}",
            "Negative parallelisms: \u{201c}not just X, but (also) Y\u{201d}, \u{201c}not only \u{2026} but \u{2026}\u{201d}, and the \u{201c}it\u{2019}s not X, it\u{2019}s Y\u{201d} contrast.",
            "Keep only the second half of the contrast.",
            rx(
                r"(?i)\bnot\s+(?:just|only|merely|simply)\s+[^.!?\n;]*?\bbut(?:\s+also)?\b|\b(?:it|this|that)(?:['\x{2019}]s|\s+(?:is|was))\s+not\s+[^.!?\n,;\x{2014}\x{2013}]{1,60}[,;\x{2014}\x{2013}]\s*(?:it|this|that)(?:['\x{2019}]s|\s+(?:is|was))\b",
            ),
        ),
        w(
            "note-that",
            "\u{201c}It\u{2019}s important to note\u{201d}",
            "Didactic hedging: \u{201c}it is important to note that\u{201d}, \u{201c}it\u{2019}s worth noting\u{201d}, \u{201c}it should be noted\u{201d}, plus the \u{201c}worth pausing / considering / asking\u{201d} family.",
            "Delete the hedge and state the fact.",
            rx(
                r"(?i)\bit(?:['\x{2019}]s|\s+(?:is|was))\s+(?:also\s+)?(?:important|worth|crucial|essential|vital)\s+(?:to\s+(?:note|remember|understand|recognize|mention|pause|consider|ask)|noting|mentioning|remembering|pausing|considering|asking)\b(?:\s+that\b)?|\bit\s+should\s+be\s+noted\b",
            ),
        ),
        w(
            "testament",
            "\u{201c}Stands as a testament\u{201d}",
            "\u{201c}Stands / serves as a testament (or reminder)\u{201d}, \u{201c}is a testament to\u{201d}: inflating significance instead of saying what happened.",
            "Say what happened instead of what it is a testament to.",
            rx(
                r"(?i)\b(?:stand|stands|stood|serve|serves|served|standing|serving)\s+as\s+(?:a|an)\s+(?:\w+\s+)?(?:testament|reminder)\b|\b(?:is|was|are|were|remain|remains)\s+a\s+(?:\w+\s+)?testament\s+to\b",
            ),
        ),
        w(
            "crucial-role",
            "\u{201c}Plays a crucial role\u{201d}",
            "\u{201c}Plays a crucial / pivotal / vital / key / significant role in \u{2026}\u{201d}.",
            "Say what it does instead of calling its role crucial.",
            rx(
                r"(?i)\bplay(?:s|ed|ing)?\s+(?:a|an)\s+(?:\w+\s+)?(?:crucial|pivotal|vital|key|significant|central|critical|important)\s+role\b",
            ),
        ),
        w(
            "landscape",
            "\u{201c}Ever-evolving landscape\u{201d}",
            "Scene-setting boilerplate: \u{201c}the ever-evolving / changing / shifting landscape\u{201d}, \u{201c}in today\u{2019}s fast-paced world\u{201d}.",
            "Delete the scene-setting and start with the specific subject.",
            rx(
                r"(?i)\b(?:ever-)?(?:evolving|changing|shifting)\s+landscape\b|\bin\s+today['\x{2019}]s\s+(?:fast-paced|ever-changing|ever-evolving|digital|modern|competitive)\s+\w+",
            ),
        ),
        w(
            "vague-experts",
            "\u{201c}Experts argue\u{201d}",
            "Vague attribution to unnamed authorities: \u{201c}experts argue\u{201d}, \u{201c}some critics have noted\u{201d}, \u{201c}observers suggest\u{201d}, \u{201c}industry reports indicate\u{201d}.",
            "Name the source or drop the attribution.",
            rx(
                r"(?i)\b(?:many|some|several|most|numerous)?\s*(?:experts|critics|observers|scholars|analysts|commentators)\s+(?:have\s+|often\s+|widely\s+)?(?:argu(?:e|es|ed)|not(?:e|es|ed)|suggest(?:s|ed)?|believ(?:e|es|ed)|agree[ds]?|contend(?:s|ed)?|observ(?:e|es|ed)|caution(?:s|ed)?|claim(?:s|ed)?|cit(?:e|es|ed)|point(?:s|ed)?\s+out)\b|\bindustry\s+reports?\s+(?:suggest|indicate|show)\w*\b",
            ),
        ),
        w(
            "despite-challenges",
            "\u{201c}Despite these challenges\u{201d}",
            "The boilerplate challenges-and-outlook formula: \u{201c}despite these challenges\u{201d}, \u{201c}faces several challenges\u{201d}, \u{201c}challenges remain\u{201d}, \u{201c}remains to be seen\u{201d}, \u{201c}time will tell\u{201d}.",
            "Name the specific challenge or cut the sentence.",
            rx(
                r"(?i)\bdespite\s+(?:these|those|such|its|their|the|numerous|significant|ongoing)\s+(?:\w+\s+)?challenges\b|\bfac(?:e|es|ed|ing)\s+(?:several|numerous|many|significant|various|a\s+number\s+of)\s+challenges\b|\bchallenges\s+remain\b|\bremains\s+to\s+be\s+seen\b|\b(?:only\s+)?time\s+will\s+tell\b",
            ),
        ),
        w(
            "participle-tail",
            "Participle sentence tails",
            "Superficial analysis bolted onto a sentence end: \u{201c}\u{2026}, highlighting / underscoring / showcasing / reflecting the \u{2026}\u{201d}.",
            "End the sentence before the comma, or make the tail its own sentence with a subject.",
            rx(
                r"(?i),\s+(?:highlighting|underscoring|emphasizing|showcasing|reflecting|demonstrating|illustrating|signaling|solidifying|cementing|reinforcing|underlining)\s+(?:its|his|her|their|our|the|a|an|how|that|what|both)\b[^.!?\n]*",
            ),
        ),
        w(
            "promo",
            "Promotional boilerplate",
            "Travel-brochure tone: \u{201c}nestled in\u{201d}, \u{201c}in the heart of\u{201d}, \u{201c}rich tapestry / heritage\u{201d}, \u{201c}hidden gem\u{201d}, \u{201c}boasts a\u{201d}, \u{201c}breathtaking\u{201d}, \u{201c}stunning views\u{201d}.",
            "Replace with a factual description.",
            rx(
                r"(?i)\bnestled\s+(?:in|on|among|between|along|at)\b|\bin\s+the\s+heart\s+of\b|\brich\s+(?:cultural\s+|historical\s+)?(?:heritage|history|tapestry)\b|\bhidden\s+gem\b|\bmust-(?:visit|see|try)\b|\bbreathtaking\b|\bboasts?\s+(?:a|an|the)\b|\bstunning\s+(?:views?|scenery|architecture|backdrop)\b",
            ),
        ),
        w(
            "ai-leftovers",
            "Chatbot leftovers",
            "Artifacts pasted straight from a chatbot: \u{201c}as an AI language model\u{201d}, \u{201c}as of my last update\u{201d}, \u{201c}knowledge cutoff\u{201d}, plus markup debris like \u{201c}oaicite\u{201d}, \u{201c}contentReference\u{201d}, \u{201c}turn0search\u{201d} and \u{201c}utm_source=\u{201d} tracking parameters.",
            "Delete the artifact.",
            rx(
                r"(?i)\bas\s+an\s+ai(?:\s+language)?\s+model\b|\bas\s+of\s+my\s+last\s+(?:update|training)\b|\bknowledge\s+cutoff\b|\bI\s+(?:cannot|can['\x{2019}]t|do\s+not|don['\x{2019}]t)\s+(?:browse\s+the\s+internet|access\s+real-?time)\b|contentReference|oaicite|turn0(?:search|news|image)\d*|attributableIndex|utm_source=",
            ),
        ),
    ]
}
