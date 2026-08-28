use clap::{Parser, ValueEnum};
use deslop::{EXAMPLE, Match, PATTERNS, Report, analyze, build_windows, snippet};
use ignore::WalkBuilder;
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SCHEMA_VERSION: u32 = 1;
const DEFAULT_EXTS: &str = "md,txt,mdx,rst";

const AGENT_HELP: &str = "\
deslop for agents

Loop:
  1. deslop --jsonl PATH_OR_DIR        (or: deslop --jsonl --text \"draft\")
  2. For each line, rewrite the `sentence` following `hint`. Do not just delete
     the matched words; restate the claim plainly.
  3. deslop --check --quiet PATH       exit 0 means clean, 1 means matches remain.
  4. Repeat from 1 until exit 0. Two or three passes is normal.

Inputs:
  file, directory (walked, .gitignore respected, extensions md,txt,mdx,rst by
  default, override with --ext), \"-\" for stdin, or --text \"...\" for inline prose.

JSONL fields:
  path, pattern, name, hint, line, col, start, end, text, sentence, version.
  `count` appears on chain patterns, `note` when the detector has extra context.
  `version` is the schema version, currently 1. Pin on it.

Tuning:
  --skip colon-triple      for technical docs where colon lists are legitimate
  --only ai-vocab,not-just to gate on a subset
  --list-patterns --json   full catalogue with description and hint per id

Exit codes: 0 ok, 1 matches found with --check, 2 usage or IO error.
";

const AFTER_HELP: &str = "\
Exit codes:
  0  ran successfully (matches or not)
  1  --check was given and at least one match was found
  2  bad arguments, unreadable file, or unknown pattern id

Output (text):
  One line per match, grep-style, then a summary block:
    PATH:LINE:COL: PATTERN-ID  \u{201c}matched text\u{201d}  (note)
  Lines and columns are 1-based; columns count characters.

Examples:
  deslop draft.md                      audit one file
  deslop docs/                         walk a directory (.gitignore respected)
  deslop --text \"Turns out it works.\"  audit inline prose
  cat draft.md | deslop                audit stdin
  deslop --json draft.md | jq .        machine-readable, includes offsets and sentences
  deslop --jsonl a.md b.md             one JSON object per match, streams well
  deslop --excerpt draft.md            flagged sentences with 12 words of context
  deslop --skip colon-triple docs/*.md drop a noisy pattern for technical docs
  deslop --only ai-vocab,not-just x.md run a subset
  deslop --check --quiet README.md     gate: exit 1 when anything matches, no output
  deslop --agent-help                  the run/rewrite/recheck loop for agents
  deslop --list-patterns               what each pattern id means
  deslop --example | deslop            demo text that trips every pattern";

#[derive(Parser)]
#[command(
    name = "deslop",
    version,
    about = "Audit prose for LLM clich\u{e9}s and print the findings to stdout.",
    long_about = "Audit prose for LLM clich\u{e9}s and print the findings to stdout.\n\n\
        Scans the input for 38 known tells (\u{201c}no X, no Y\u{201d} chains, \u{201c}that\u{2019}s the whole point\u{201d}, \
        \u{201c}delve\u{201d}-class vocabulary, stacked rhetorical questions, echoing sentence skeletons, and the \
        patterns from Wikipedia\u{2019}s \u{201c}Signs of AI writing\u{201d}). Overlapping hits are resolved to one match \
        each, and each match is mapped to the sentence that contains it.",
    after_help = AFTER_HELP
)]
struct Cli {
    /// Files or directories to audit. Directories are walked (.gitignore respected). Use "-" or pass nothing to read stdin.
    #[arg(value_name = "PATH")]
    files: Vec<PathBuf>,

    /// Audit this text instead of files. Repeatable.
    #[arg(long, value_name = "TEXT", conflicts_with = "files")]
    text: Vec<String>,

    /// Extensions to pick up when walking a directory (comma-separated).
    #[arg(long, value_delimiter = ',', value_name = "EXT", default_value = DEFAULT_EXTS)]
    ext: Vec<String>,

    /// Output format.
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,

    /// Shortcut for --format json.
    #[arg(long, conflicts_with_all = ["jsonl", "format"])]
    json: bool,

    /// Shortcut for --format jsonl.
    #[arg(long, conflicts_with_all = ["json", "format"])]
    jsonl: bool,

    /// Run only these pattern ids (comma-separated or repeated).
    #[arg(long, value_delimiter = ',', value_name = "ID")]
    only: Vec<String>,

    /// Skip these pattern ids (comma-separated or repeated).
    #[arg(long, value_delimiter = ',', value_name = "ID")]
    skip: Vec<String>,

    /// Text mode: print flagged sentences with surrounding context instead of one line per match.
    #[arg(long)]
    excerpt: bool,

    /// Text mode: print only the summary block.
    #[arg(long, conflicts_with = "excerpt")]
    summary_only: bool,

    /// Exit 1 when any match is found.
    #[arg(long)]
    check: bool,

    /// Print nothing; useful with --check when only the exit code matters.
    #[arg(long, short)]
    quiet: bool,

    /// Print the run/rewrite/recheck loop for agents, then exit.
    #[arg(long)]
    agent_help: bool,

    /// List every pattern id with its name and description, then exit.
    #[arg(long)]
    list_patterns: bool,

    /// Print the built-in example text (trips every pattern once), then exit.
    #[arg(long)]
    example: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, ValueEnum)]
enum Format {
    Text,
    Json,
    Jsonl,
}

#[derive(Serialize)]
struct MatchOut<'a> {
    path: &'a str,
    pattern: &'static str,
    name: &'static str,
    hint: &'static str,
    line: usize,
    col: usize,
    start: usize,
    end: usize,
    text: &'a str,
    sentence: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<&'a str>,
    version: u32,
}

#[derive(Serialize)]
struct Summary {
    matches: usize,
    flagged_sentences: usize,
    chain_items: usize,
    per_pattern: BTreeMap<&'static str, usize>,
}

#[derive(Serialize)]
struct FileOut<'a> {
    path: &'a str,
    matches: Vec<MatchOut<'a>>,
    summary: Summary,
}

#[derive(Serialize)]
struct PatternOut {
    id: &'static str,
    name: &'static str,
    group: Option<&'static str>,
    description: &'static str,
    hint: &'static str,
}

struct Doc {
    path: String,
    text: String,
    report: Report,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("deslop: {e}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = Cli::parse();
    let format = if cli.json {
        Format::Json
    } else if cli.jsonl {
        Format::Jsonl
    } else {
        cli.format
    };
    let mut out = io::stdout().lock();
    let emit = |out: &mut io::StdoutLock, s: String| -> Result<(), String> {
        out.write_all(s.as_bytes()).map_err(|e| e.to_string())
    };

    if cli.example {
        emit(&mut out, EXAMPLE.to_string())?;
        return Ok(ExitCode::SUCCESS);
    }

    if cli.agent_help {
        emit(&mut out, AGENT_HELP.to_string())?;
        return Ok(ExitCode::SUCCESS);
    }

    if cli.list_patterns {
        let text = if format == Format::Text {
            let mut s = String::new();
            for p in PATTERNS.iter() {
                s.push_str(&format!("{}\n  {}\n", p.id, p.name));
                if let Some(g) = p.group {
                    s.push_str(&format!("  group: {g}\n"));
                }
                s.push_str(&format!("  {}\n  fix: {}\n\n", p.description, p.hint));
            }
            s
        } else {
            let list: Vec<PatternOut> = PATTERNS
                .iter()
                .map(|p| PatternOut {
                    id: p.id,
                    name: p.name,
                    group: p.group,
                    description: p.description,
                    hint: p.hint,
                })
                .collect();
            if format == Format::Json {
                serde_json::to_string_pretty(&list).unwrap() + "\n"
            } else {
                list.iter()
                    .map(|p| serde_json::to_string(p).unwrap() + "\n")
                    .collect()
            }
        };
        emit(&mut out, text)?;
        return Ok(ExitCode::SUCCESS);
    }

    let enabled = enabled_patterns(&cli.only, &cli.skip)?;
    let docs = read_inputs(&cli.files, &cli.text, &cli.ext)?
        .into_iter()
        .map(|(path, text)| {
            let report = analyze(&text, &enabled);
            Doc { path, text, report }
        })
        .collect::<Vec<_>>();

    if !cli.quiet {
        let rendered = match format {
            Format::Text if cli.excerpt => render_excerpts(&docs),
            Format::Text => render_text(&docs, cli.summary_only),
            Format::Json => render_json(&docs),
            Format::Jsonl => render_jsonl(&docs),
        };
        emit(&mut out, rendered)?;
    }

    let any = docs.iter().any(|d| !d.report.matches.is_empty());
    Ok(if cli.check && any {
        ExitCode::from(1)
    } else {
        ExitCode::SUCCESS
    })
}

fn enabled_patterns(only: &[String], skip: &[String]) -> Result<HashSet<&'static str>, String> {
    let known: HashSet<&'static str> = PATTERNS.iter().map(|p| p.id).collect();
    for id in only.iter().chain(skip) {
        if !known.contains(id.as_str()) {
            return Err(format!(
                "unknown pattern id \u{201c}{id}\u{201d} (see --list-patterns)"
            ));
        }
    }
    let mut enabled: HashSet<&'static str> = if only.is_empty() {
        known.clone()
    } else {
        known
            .iter()
            .copied()
            .filter(|id| only.iter().any(|o| o == id))
            .collect()
    };
    enabled.retain(|id| !skip.iter().any(|s| s == id));
    Ok(enabled)
}

fn read_inputs(
    files: &[PathBuf],
    texts: &[String],
    exts: &[String],
) -> Result<Vec<(String, String)>, String> {
    if !texts.is_empty() {
        return Ok(texts
            .iter()
            .enumerate()
            .map(|(i, t)| (format!("<text{}>", i + 1), t.clone()))
            .collect());
    }
    if files.is_empty() {
        if io::stdin().is_terminal() {
            return Err("no input: pass a path, --text, or pipe text on stdin (try --help)".into());
        }
        return Ok(vec![("<stdin>".into(), read_stdin()?)]);
    }
    let mut docs = Vec::new();
    for f in files {
        if f.as_os_str() == "-" {
            docs.push(("<stdin>".into(), read_stdin()?));
        } else if f.is_dir() {
            for path in walk_dir(f, exts)? {
                docs.push((path.display().to_string(), read_file(&path)?));
            }
        } else {
            docs.push((f.display().to_string(), read_file(f)?));
        }
    }
    Ok(docs)
}

fn read_file(path: &Path) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn walk_dir(dir: &Path, exts: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in WalkBuilder::new(dir).build() {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_some_and(|t| t.is_file()) {
            continue;
        }
        let ext = entry.path().extension().and_then(|e| e.to_str());
        if ext.is_some_and(|e| exts.iter().any(|x| x.eq_ignore_ascii_case(e))) {
            paths.push(entry.into_path());
        }
    }
    paths.sort();
    Ok(paths)
}

fn read_stdin() -> Result<String, String> {
    let mut s = String::new();
    io::stdin()
        .read_to_string(&mut s)
        .map_err(|e| format!("stdin: {e}"))?;
    Ok(s)
}

fn line_col(text: &str, offset: usize) -> (usize, usize) {
    let before = &text[..offset];
    let line = before.matches('\n').count() + 1;
    let line_start = before.rfind('\n').map_or(0, |i| i + 1);
    (line, text[line_start..offset].chars().count() + 1)
}

fn sentence_for(doc: &Doc, index: usize) -> &str {
    let r = doc
        .report
        .regions
        .iter()
        .find(|r| r.matches.contains(&index))
        .expect("every match has a region");
    &doc.text[r.start..r.end]
}

fn match_out<'a>(doc: &'a Doc, index: usize, m: &'a Match) -> MatchOut<'a> {
    let (line, col) = line_col(&doc.text, m.start);
    MatchOut {
        path: &doc.path,
        pattern: m.pattern,
        name: deslop::pattern(m.pattern).unwrap().name,
        hint: deslop::pattern(m.pattern).unwrap().hint,
        line,
        col,
        start: m.start,
        end: m.end,
        text: &doc.text[m.start..m.end],
        sentence: sentence_for(doc, index),
        count: m.count,
        note: m.note.as_deref(),
        version: SCHEMA_VERSION,
    }
}

fn summary(report: &Report) -> Summary {
    Summary {
        matches: report.matches.len(),
        flagged_sentences: report.regions.len(),
        chain_items: report.chain_items(),
        per_pattern: report
            .per_pattern
            .iter()
            .filter(|(_, n)| **n > 0)
            .map(|(k, v)| (*k, *v))
            .collect(),
    }
}

fn plural<'a>(n: usize, one: &'a str, many: &'a str) -> &'a str {
    if n == 1 { one } else { many }
}

fn summary_text(doc: &Doc) -> String {
    let s = summary(&doc.report);
    let mut out = format!(
        "{}: {} {}, {} flagged {}, {} chain {}\n",
        doc.path,
        s.matches,
        plural(s.matches, "match", "matches"),
        s.flagged_sentences,
        plural(s.flagged_sentences, "sentence", "sentences"),
        s.chain_items,
        plural(s.chain_items, "item", "items"),
    );
    let mut by_count: Vec<(&str, usize)> = s.per_pattern.into_iter().collect();
    by_count.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    for (id, n) in by_count {
        out.push_str(&format!("  {n:>3}  {id}\n"));
    }
    out
}

fn render_text(docs: &[Doc], summary_only: bool) -> String {
    let mut out = String::new();
    if !summary_only {
        for doc in docs {
            for m in &doc.report.matches {
                let (line, col) = line_col(&doc.text, m.start);
                out.push_str(&format!(
                    "{}:{line}:{col}: {}  \u{201c}{}\u{201d}",
                    doc.path,
                    m.pattern,
                    snippet(&doc.text[m.start..m.end])
                ));
                if let Some(note) = &m.note {
                    out.push_str(&format!("  ({note})"));
                }
                out.push_str(&format!(
                    "\n    fix: {}\n",
                    deslop::pattern(m.pattern).unwrap().hint
                ));
            }
        }
        if docs.iter().any(|d| !d.report.matches.is_empty()) {
            out.push('\n');
        }
    }
    for doc in docs {
        out.push_str(&summary_text(doc));
    }
    out
}

fn render_excerpts(docs: &[Doc]) -> String {
    let mut out = String::new();
    for doc in docs {
        let text = &doc.text;
        let windows = build_windows(text, &doc.report.regions);
        let mut cursor = 0;
        for w in &windows {
            let hidden = deslop::count_words(&text[cursor..w.start]);
            if hidden > 0 {
                out.push_str(&format!(
                    "[\u{2026} {hidden} {} \u{2026}]\n",
                    plural(hidden, "word", "words")
                ));
            }
            let mut pos = w.start;
            let mut body = String::new();
            for &ri in &w.regions {
                let r = &doc.report.regions[ri];
                body.push_str(&text[pos..r.start]);
                body.push_str(">>");
                let mut inner = r.start;
                for &mi in &r.matches {
                    let m = &doc.report.matches[mi];
                    body.push_str(&text[inner..m.start]);
                    body.push_str(&format!("[[{}]]{{{}", &text[m.start..m.end], m.pattern));
                    if let Some(c) = m.count {
                        body.push_str(&format!(" x{c}"));
                    }
                    body.push('}');
                    inner = m.end;
                }
                body.push_str(&text[inner..r.end]);
                body.push_str("<<");
                pos = r.end;
            }
            body.push_str(&text[pos..w.end]);
            out.push_str(&format!(
                "{}:{}:\n{}\n\n",
                doc.path,
                line_col(text, w.start).0,
                body.trim_end()
            ));
            cursor = w.end;
        }
        let hidden = deslop::count_words(&text[cursor..]);
        if hidden > 0 && !windows.is_empty() {
            out.push_str(&format!(
                "[\u{2026} {hidden} {} \u{2026}]\n\n",
                plural(hidden, "word", "words")
            ));
        }
        out.push_str(&summary_text(doc));
    }
    out
}

fn render_json(docs: &[Doc]) -> String {
    let files: Vec<FileOut> = docs
        .iter()
        .map(|doc| FileOut {
            path: &doc.path,
            matches: doc
                .report
                .matches
                .iter()
                .enumerate()
                .map(|(i, m)| match_out(doc, i, m))
                .collect(),
            summary: summary(&doc.report),
        })
        .collect();
    let total = Summary {
        matches: files.iter().map(|f| f.summary.matches).sum(),
        flagged_sentences: files.iter().map(|f| f.summary.flagged_sentences).sum(),
        chain_items: files.iter().map(|f| f.summary.chain_items).sum(),
        per_pattern: files
            .iter()
            .flat_map(|f| f.summary.per_pattern.iter())
            .fold(BTreeMap::new(), |mut acc, (k, v)| {
                *acc.entry(*k).or_insert(0) += v;
                acc
            }),
    };
    #[derive(Serialize)]
    struct Root<'a> {
        version: u32,
        files: Vec<FileOut<'a>>,
        total: Summary,
    }
    serde_json::to_string_pretty(&Root {
        version: SCHEMA_VERSION,
        files,
        total,
    })
    .unwrap()
        + "\n"
}

fn render_jsonl(docs: &[Doc]) -> String {
    let mut out = String::new();
    for doc in docs {
        for (i, m) in doc.report.matches.iter().enumerate() {
            out.push_str(&serde_json::to_string(&match_out(doc, i, m)).unwrap());
            out.push('\n');
        }
    }
    out
}
