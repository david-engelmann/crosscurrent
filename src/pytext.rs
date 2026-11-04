//! The text model the reader stands on, taken from the
//! oracle runtime's own character data.
//!
//! The Python implementation strips, splits, and divides its
//! texts with the runtime's Unicode whitespace set, its line
//! boundary set, and, in the argument pattern, its decimal
//! digit set. Those sets are wider than the ASCII sets the
//! Rust standard library reads, so they stand here as tables
//! probed off the oracle runtime before the build, and the
//! operations the reader uses are written over them (XC-27
//! and XC-28).

/// The whitespace ranges, code points low to high.
pub const WHITESPACE_RANGES: &[(u32, u32)] = &[
    (0x0009, 0x000D),
    (0x001C, 0x0020),
    (0x0085, 0x0085),
    (0x00A0, 0x00A0),
    (0x1680, 0x1680),
    (0x2000, 0x200A),
    (0x2028, 0x2029),
    (0x202F, 0x202F),
    (0x205F, 0x205F),
    (0x3000, 0x3000),
];

/// The decimal digit ranges, code points low to high.
pub const DECIMAL_RANGES: &[(u32, u32)] = &[
    (0x0030, 0x0039),
    (0x0660, 0x0669),
    (0x06F0, 0x06F9),
    (0x07C0, 0x07C9),
    (0x0966, 0x096F),
    (0x09E6, 0x09EF),
    (0x0A66, 0x0A6F),
    (0x0AE6, 0x0AEF),
    (0x0B66, 0x0B6F),
    (0x0BE6, 0x0BEF),
    (0x0C66, 0x0C6F),
    (0x0CE6, 0x0CEF),
    (0x0D66, 0x0D6F),
    (0x0DE6, 0x0DEF),
    (0x0E50, 0x0E59),
    (0x0ED0, 0x0ED9),
    (0x0F20, 0x0F29),
    (0x1040, 0x1049),
    (0x1090, 0x1099),
    (0x17E0, 0x17E9),
    (0x1810, 0x1819),
    (0x1946, 0x194F),
    (0x19D0, 0x19D9),
    (0x1A80, 0x1A89),
    (0x1A90, 0x1A99),
    (0x1B50, 0x1B59),
    (0x1BB0, 0x1BB9),
    (0x1C40, 0x1C49),
    (0x1C50, 0x1C59),
    (0xA620, 0xA629),
    (0xA8D0, 0xA8D9),
    (0xA900, 0xA909),
    (0xA9D0, 0xA9D9),
    (0xA9F0, 0xA9F9),
    (0xAA50, 0xAA59),
    (0xABF0, 0xABF9),
    (0xFF10, 0xFF19),
    (0x104A0, 0x104A9),
    (0x10D30, 0x10D39),
    (0x11066, 0x1106F),
    (0x110F0, 0x110F9),
    (0x11136, 0x1113F),
    (0x111D0, 0x111D9),
    (0x112F0, 0x112F9),
    (0x11450, 0x11459),
    (0x114D0, 0x114D9),
    (0x11650, 0x11659),
    (0x116C0, 0x116C9),
    (0x11730, 0x11739),
    (0x118E0, 0x118E9),
    (0x11950, 0x11959),
    (0x11C50, 0x11C59),
    (0x11D50, 0x11D59),
    (0x11DA0, 0x11DA9),
    (0x11F50, 0x11F59),
    (0x16A60, 0x16A69),
    (0x16AC0, 0x16AC9),
    (0x16B50, 0x16B59),
    (0x1D7CE, 0x1D7FF),
    (0x1E140, 0x1E149),
    (0x1E2F0, 0x1E2F9),
    (0x1E4F0, 0x1E4F9),
    (0x1E950, 0x1E959),
    (0x1FBF0, 0x1FBF9),
];

fn in_ranges(ranges: &[(u32, u32)], c: char) -> bool {
    let point = c as u32;
    ranges
        .binary_search_by(|&(low, high)| {
            if point < low {
                std::cmp::Ordering::Greater
            } else if point > high {
                std::cmp::Ordering::Less
            } else {
                std::cmp::Ordering::Equal
            }
        })
        .is_ok()
}

/// Whether the character is whitespace in the oracle's set.
pub fn is_space(c: char) -> bool {
    in_ranges(WHITESPACE_RANGES, c)
}

/// Whether the character is a decimal digit in the oracle's
/// set, the set the argument pattern's digit class reads.
pub fn is_decimal(c: char) -> bool {
    in_ranges(DECIMAL_RANGES, c)
}

/// Strip the oracle's whitespace from both ends of a text,
/// as the runtime's strip does with no arguments.
pub fn strip(text: &str) -> &str {
    text.trim_matches(is_space)
}

/// Split a text over runs of the oracle's whitespace, with
/// no empty pieces, as the runtime's split does with no
/// arguments.
pub fn split(text: &str) -> Vec<&str> {
    let mut pieces = Vec::new();
    let mut start: Option<usize> = None;
    for (index, c) in text.char_indices() {
        if is_space(c) {
            if let Some(begin) = start.take() {
                pieces.push(&text[begin..index]);
            }
        } else if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(begin) = start {
        pieces.push(&text[begin..]);
    }
    pieces
}

/// Whether the character ends a line in the oracle's line
/// boundary set.
fn is_boundary(c: char) -> bool {
    matches!(
        c,
        '\n' | '\r'
            | '\u{b}'
            | '\u{c}'
            | '\u{1c}'
            | '\u{1d}'
            | '\u{1e}'
            | '\u{85}'
            | '\u{2028}'
            | '\u{2029}'
    )
}

/// Divide a text into lines over the oracle's boundary set,
/// as the runtime's splitlines does. A carriage return and
/// line feed pair is one boundary, and a text that ends on a
/// boundary yields no empty final piece.
pub fn splitlines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut chars = text.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if !is_boundary(c) {
            continue;
        }
        lines.push(&text[start..index]);
        let mut end = index + c.len_utf8();
        if c == '\r' && matches!(chars.peek(), Some((_, '\n'))) {
            let (next_index, next) = chars.next().expect("peeked char");
            end = next_index + next.len_utf8();
        }
        start = end;
    }
    if start < text.len() {
        lines.push(&text[start..]);
    }
    lines
}
