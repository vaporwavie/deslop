pub(crate) fn char_before(text: &str, i: usize) -> Option<char> {
    text[..i].chars().next_back()
}

pub(crate) fn char_at(text: &str, i: usize) -> Option<char> {
    text[i..].chars().next()
}

pub(crate) fn trim_end_ws(text: &str, floor: usize, mut end: usize) -> usize {
    while end > floor {
        match char_before(text, end) {
            Some(c) if c.is_whitespace() => end -= c.len_utf8(),
            _ => break,
        }
    }
    end
}

pub(crate) fn trim_start_ws(text: &str, mut start: usize, ceil: usize) -> usize {
    while start < ceil {
        match char_at(text, start) {
            Some(c) if c.is_whitespace() => start += c.len_utf8(),
            _ => break,
        }
    }
    start
}
