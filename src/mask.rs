use regex::Regex;
use std::sync::LazyLock;

// Blank every span that is not prose (fenced code, inline code, link targets,
// bare URLs, blockquotes, HTML entities) so no pattern fires inside it. Byte
// length is preserved: every masked byte becomes a space, newlines stay, so
// match offsets index the original text unchanged.
pub fn mask_markdown(text: &str) -> String {
    let mut out = text.as_bytes().to_vec();
    let mut fence: Option<(u8, usize)> = None;
    let mut frontmatter = text.starts_with("---\n");
    let mut pos = 0;
    for line in text.split_inclusive('\n') {
        let start = pos;
        pos += line.len();
        let body = line.trim_end_matches('\n');
        let end = start + body.len();
        if frontmatter {
            blank(&mut out, start, end);
            if start > 0 && body == "---" {
                frontmatter = false;
            }
            continue;
        }
        let trimmed = body.trim_start_matches(' ');
        let indent = body.len() - trimmed.len();
        if indent <= 3
            && let Some((ch, len)) = fence_run(trimmed)
        {
            match fence {
                Some((open, open_len))
                    if ch == open && len >= open_len && trimmed[len..].trim().is_empty() =>
                {
                    blank(&mut out, start, end);
                    fence = None;
                    continue;
                }
                None => {
                    blank(&mut out, start, end);
                    fence = Some((ch, len));
                    continue;
                }
                Some(_) => {}
            }
        }
        if fence.is_some() || (indent <= 3 && trimmed.starts_with('>')) {
            blank(&mut out, start, end);
            continue;
        }
        mask_inline(body, &mut out, start);
    }
    String::from_utf8(out).expect("masking only writes ASCII spaces")
}

fn fence_run(line: &str) -> Option<(u8, usize)> {
    let first = *line.as_bytes().first()?;
    if first != b'`' && first != b'~' {
        return None;
    }
    let len = line.bytes().take_while(|&b| b == first).count();
    (len >= 3).then_some((first, len))
}

fn blank(out: &mut [u8], start: usize, end: usize) {
    out[start..end].fill(b' ');
}

static LINK_TARGET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\]\([^)]*\)").unwrap());
static URL: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"<?https?://[^\s<>)\]]+>?").unwrap());
static ENTITY: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"&(?:#\d+|[a-zA-Z]+);").unwrap());

fn mask_inline(line: &str, out: &mut [u8], base: usize) {
    for (s, e) in code_spans(line) {
        blank(out, base + s, base + e);
    }
    for m in LINK_TARGET.find_iter(line) {
        blank(out, base + m.start() + 1, base + m.end());
    }
    for re in [&*URL, &*ENTITY] {
        for m in re.find_iter(line) {
            blank(out, base + m.start(), base + m.end());
        }
    }
}

// Backtick code spans within one line: a run of N backticks closes at the next
// run of exactly N. An unmatched run is left alone.
fn code_spans(line: &str) -> Vec<(usize, usize)> {
    let bytes = line.as_bytes();
    let mut spans = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let open = i;
        while i < bytes.len() && bytes[i] == b'`' {
            i += 1;
        }
        let n = i - open;
        let mut j = i;
        let mut close = None;
        while j < bytes.len() {
            if bytes[j] == b'`' {
                let run = j;
                while j < bytes.len() && bytes[j] == b'`' {
                    j += 1;
                }
                if j - run == n {
                    close = Some(j);
                    break;
                }
            } else {
                j += 1;
            }
        }
        if let Some(end) = close {
            spans.push((open, end));
            i = end;
        }
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_byte_length_and_newlines() {
        let t = "Prose here.\n```js\nconst é = 1;\n```\nAfter.\n";
        let m = mask_markdown(t);
        assert_eq!(m.len(), t.len());
        assert_eq!(m.matches('\n').count(), t.matches('\n').count());
        assert!(m.starts_with("Prose here.\n"));
        assert!(m.ends_with("After.\n"));
        assert!(!m.contains("const"));
    }

    #[test]
    fn masks_inline_code_links_urls_quotes_entities() {
        let t = "Run `x; y` now. See [docs](https://e.com/a;b) or https://e.com/c;d and &amp; here.\n> quoted; text\nPlain; line.";
        let m = mask_markdown(t);
        assert!(!m.contains("x; y"));
        assert!(m.contains("[docs]"));
        assert!(!mask_markdown("[x](foo delve bar)").contains("delve"));
        assert!(!m.contains("e.com"));
        assert!(!m.contains("&amp;"));
        assert!(!m.contains("quoted"));
        assert!(m.contains("Plain; line."));
    }

    #[test]
    fn yaml_frontmatter_is_masked() {
        let t = "---\nname: x\nharness: [claude, codex, hermes]\n---\nTurns out prose; here.\n";
        let m = mask_markdown(t);
        assert!(!m.contains("harness"));
        assert!(m.contains("Turns out prose; here."));
        assert!(mask_markdown("Prose first.\n---\nnot: frontmatter\n").contains("frontmatter"));
    }

    #[test]
    fn tilde_fences_and_unmatched_backticks() {
        let t = "~~~\ndelve\n~~~\nA `lone backtick stays; visible.\n";
        let m = mask_markdown(t);
        assert!(!m.contains("delve"));
        assert!(m.contains("lone backtick stays; visible."));
    }
}
