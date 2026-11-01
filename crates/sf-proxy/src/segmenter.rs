//! Single-pass clause splitter. Output slices borrow the request buffer.
//!
//! Option 2C: punctuation and trilingual discourse connectors split first.
//! A clause longer than 14 words falls back to a word window of 10 with stride 5.

const SUB_WINDOW: usize = 10;
const SUB_STRIDE: usize = 5;
const MAX_UNWINDOWED_WORDS: usize = 14;

/// Longest pattern first. `" y "` is a prefix of `" y de paso "`.
const CONNECTORS: &[&str] = &[
    " y de paso ",
    " außerdem ",
    " también ",
    " además ",
    " also ",
    " plus ",
    " pero ",
    " aber ",
    " and ",
    " but ",
    " und ",
    " or ",
    " y ",
    " o ",
];

/// Borrowed clauses. The only heap traffic is the `Vec` of slices and, for long
/// clauses, the word-span index. No `String` is allocated.
pub fn segment_prompt_clauses(prompt: &str) -> Vec<&str> {
    let bytes = prompt.as_bytes();
    let mut raw = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if !prompt.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if starts(bytes, i, b"\"\"\"") || starts(bytes, i, b"'''") {
            push_trimmed(&mut raw, prompt, start, i);
            i += 3;
            start = i;
            continue;
        }
        if starts(bytes, i, b"/*") || starts(bytes, i, b"*/") || starts(bytes, i, b"//") {
            push_trimmed(&mut raw, prompt, start, i);
            i += 2;
            start = i;
            continue;
        }
        if bytes[i] == b' ' {
            if let Some(len) = connector_len(&prompt[i..]) {
                push_trimmed(&mut raw, prompt, start, i);
                i += len;
                start = i;
                continue;
            }
        }
        let byte = bytes[i];
        let punct = byte == b'\n'
            || byte == b'\r'
            || byte == b';'
            || byte == b','
            || byte == b'#'
            || ((byte == b'.' || byte == b'!' || byte == b'?')
                && (i + 1 == bytes.len() || bytes[i + 1].is_ascii_whitespace()));
        if punct {
            push_trimmed(&mut raw, prompt, start, i);
            start = i + 1;
        }
        i += 1;
    }
    push_trimmed(&mut raw, prompt, start, bytes.len());

    let mut clauses = Vec::with_capacity(raw.len().max(1));
    for clause in raw {
        window_clause(clause, &mut clauses);
    }
    if clauses.is_empty() {
        let trimmed = prompt.trim();
        if !trimmed.is_empty() {
            clauses.push(trimmed);
        }
    }
    clauses
}

fn connector_len(haystack: &str) -> Option<usize> {
    CONNECTORS
        .iter()
        .find_map(|needle| match_ignore_case(haystack, needle))
}

fn match_ignore_case(haystack: &str, needle: &str) -> Option<usize> {
    let mut consumed = 0usize;
    let mut rest = haystack.chars();
    for expected in needle.chars() {
        let found = rest.next()?;
        if !eq_ignore_case(found, expected) {
            return None;
        }
        consumed += found.len_utf8();
    }
    Some(consumed)
}

fn eq_ignore_case(left: char, right: char) -> bool {
    if left == right {
        return true;
    }
    if left.is_ascii() && right.is_ascii() {
        return left.eq_ignore_ascii_case(&right);
    }
    let mut left_lower = left.to_lowercase();
    let mut right_lower = right.to_lowercase();
    let (Some(folded_left), Some(folded_right)) = (left_lower.next(), right_lower.next()) else {
        return false;
    };
    folded_left == folded_right && left_lower.next().is_none() && right_lower.next().is_none()
}

fn starts(bytes: &[u8], index: usize, needle: &[u8]) -> bool {
    bytes.get(index..index + needle.len()) == Some(needle)
}

fn push_trimmed<'a>(out: &mut Vec<&'a str>, prompt: &'a str, start: usize, end: usize) {
    if start >= end || end > prompt.len() {
        return;
    }
    let piece = prompt[start..end].trim();
    if !piece.is_empty() {
        out.push(piece);
    }
}

fn window_clause<'a>(clause: &'a str, out: &mut Vec<&'a str>) {
    let spans = word_spans(clause);
    if spans.len() <= MAX_UNWINDOWED_WORDS {
        out.push(clause);
        return;
    }
    let mut cursor = 0usize;
    while cursor < spans.len() {
        let end = (cursor + SUB_WINDOW).min(spans.len());
        let byte_start = spans[cursor].0;
        let byte_end = spans[end - 1].1;
        out.push(&clause[byte_start..byte_end]);
        if end == spans.len() {
            break;
        }
        cursor += SUB_STRIDE;
    }
}

fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut in_word = false;
    let mut word_start = 0usize;
    for (idx, ch) in text.char_indices() {
        if ch.is_whitespace() {
            if in_word {
                spans.push((word_start, idx));
                in_word = false;
            }
        } else if !in_word {
            word_start = idx;
            in_word = true;
        }
    }
    if in_word {
        spans.push((word_start, text.len()));
    }
    spans
}
