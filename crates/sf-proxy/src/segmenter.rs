//! Slice 1 structural splitter: newlines and sentence punctuation.
//! Docstring boundaries and the sliding window land in Slice 4.

/// Borrowed clauses over `prompt`. No `String` allocation.
pub fn segment_prompt_clauses(prompt: &str) -> Vec<&str> {
    let bytes = prompt.as_bytes();
    let mut raw = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        let byte = bytes[i];
        let punct = byte == b'\n'
            || byte == b';'
            || ((byte == b'.' || byte == b'!' || byte == b'?')
                && (i + 1 == bytes.len() || bytes[i + 1].is_ascii_whitespace()));
        if punct {
            let end = if byte == b'\n' { i } else { i + 1 };
            push_trimmed(&mut raw, prompt, start, end);
            start = i + 1;
        }
        i += 1;
    }
    push_trimmed(&mut raw, prompt, start, bytes.len());
    if raw.is_empty() {
        let trimmed = prompt.trim();
        if !trimmed.is_empty() {
            raw.push(trimmed);
        }
    }
    raw
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
