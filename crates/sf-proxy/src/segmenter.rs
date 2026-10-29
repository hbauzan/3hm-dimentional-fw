//! Single-pass clause splitter. Output slices borrow the request buffer.

const WINDOW: usize = 20;
const STRIDE: usize = 10;

/// Borrowed clauses. The only heap traffic is the `Vec` of slices and, for long
/// clauses, the word-span index. No `String` is allocated.
pub fn segment_prompt_clauses(prompt: &str) -> Vec<&str> {
    let bytes = prompt.as_bytes();
    let mut raw = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
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
        let byte = bytes[i];
        let punct = byte == b'\n'
            || byte == b'\r'
            || byte == b';'
            || byte == b'#'
            || ((byte == b'.' || byte == b'!' || byte == b'?')
                && (i + 1 == bytes.len() || bytes[i + 1].is_ascii_whitespace()));
        if punct {
            let end = if byte == b'\n' || byte == b'\r' || byte == b'#' {
                i
            } else {
                i + 1
            };
            push_trimmed(&mut raw, prompt, start, end);
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
    if spans.len() <= WINDOW {
        out.push(clause);
        return;
    }
    let mut cursor = 0usize;
    while cursor < spans.len() {
        let end = (cursor + WINDOW).min(spans.len());
        let byte_start = spans[cursor].0;
        let byte_end = spans[end - 1].1;
        out.push(&clause[byte_start..byte_end]);
        if end == spans.len() {
            break;
        }
        cursor += STRIDE;
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
