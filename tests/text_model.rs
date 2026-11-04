//! The text model pins (XC-27 and XC-28). The character
//! tables are pinned at every range edge and just outside
//! it, the line boundary set is pinned character by
//! character, and end to end parses exercise the boundaries
//! the tables name.

mod common;

use common::{state, state_err};
use crosscurrent::pytext::{is_decimal, is_space, splitlines};

#[test]
fn whitespace_table_edges() {
    let inside = [
        '\u{9}', '\u{a}', '\u{b}', '\u{c}', '\u{d}', ' ', '\u{1c}', '\u{1d}', '\u{1e}', '\u{1f}',
        '\u{85}', '\u{a0}', '\u{1680}', '\u{2000}', '\u{2005}', '\u{200a}', '\u{2028}', '\u{2029}',
        '\u{202f}', '\u{205f}', '\u{3000}',
    ];
    for ch in inside {
        assert!(is_space(ch), "U+{:04X} must read as whitespace", ch as u32);
    }
    let outside = [
        '\u{8}', '\u{e}', '\u{1b}', '\u{21}', '\u{84}', '\u{86}', '\u{a1}', '\u{167f}', '\u{1681}',
        '\u{200b}', '\u{202a}', '\u{202e}', '\u{2030}', '\u{205e}', '\u{2060}', '\u{3001}',
        '\u{feff}', 'a', '0',
    ];
    for ch in outside {
        assert!(
            !is_space(ch),
            "U+{:04X} must not read as whitespace",
            ch as u32
        );
    }
}

#[test]
fn decimal_table_edges() {
    let inside = [
        '0',
        '9',
        '\u{660}',
        '\u{669}',
        '\u{6f0}',
        '\u{6f9}',
        '\u{7c0}',
        '\u{7c9}',
        '\u{966}',
        '\u{96f}',
        '\u{9e6}',
        '\u{9ef}',
        '\u{a620}',
        '\u{a629}',
        '\u{ff10}',
        '\u{ff19}',
        '\u{104a0}',
        '\u{104a9}',
        '\u{1d7ce}',
        '\u{1d7d8}',
        '\u{1d7ff}',
    ];
    for ch in inside {
        assert!(is_decimal(ch), "U+{:04X} must read as decimal", ch as u32);
    }
    let outside = [
        '\u{65f}',
        '\u{66a}',
        '\u{6ef}',
        '\u{6fa}',
        '\u{7bf}',
        '\u{7ca}',
        '\u{965}',
        '\u{970}',
        '\u{a61f}',
        '\u{a62a}',
        '\u{ff0f}',
        '\u{ff1a}',
        '\u{1049f}',
        '\u{104aa}',
        'a',
        '.',
        ' ',
    ];
    for ch in outside {
        assert!(
            !is_decimal(ch),
            "U+{:04X} must not read as decimal",
            ch as u32
        );
    }
}

#[test]
fn splitlines_honours_every_boundary_character() {
    let boundaries = [
        '\n', '\r', '\u{b}', '\u{c}', '\u{1c}', '\u{1d}', '\u{1e}', '\u{85}', '\u{2028}',
        '\u{2029}',
    ];
    for boundary in boundaries {
        let text = format!("left{boundary}right");
        assert_eq!(
            splitlines(&text),
            ["left", "right"],
            "U+{:04X} must split lines",
            boundary as u32
        );
    }
    assert_eq!(splitlines("a\r\nb"), ["a", "b"], "CRLF is one boundary");
    assert_eq!(
        splitlines("a\n"),
        ["a"],
        "a trailing boundary adds no piece"
    );
    assert_eq!(splitlines(""), Vec::<&str>::new());
    assert_eq!(splitlines("a\n\nb"), ["a", "", "b"]);
}

#[test]
fn line_numbers_count_raw_splitlines_positions() {
    // The vertical tab is a line boundary, so the malformed
    // line stands at position 3, after the interface line
    // and the author line it follows on the page.
    let err = state_err(
        "interface shard-store\nauthor engineer-a\u{b}bogus line\n",
        "run.state",
    );
    let message = err.to_string();
    assert!(
        message.contains("line 3 matches no statement form"),
        "{message}"
    );
    // A CRLF pair is one boundary, so the malformed line
    // after two CRLF ended lines stands at position 3 and
    // not at the position counting the pair as two.
    let err = state_err(
        "interface shard-store\r\nauthor engineer-a\r\nbogus line\n",
        "run.state",
    );
    let message = err.to_string();
    assert!(
        message.contains("line 3 matches no statement form"),
        "{message}"
    );
}

#[test]
fn non_breaking_space_separates_words() {
    let parsed = state(
        "interface\u{a0}shard-store\nauthor engineer-a\n",
        "run.state",
    );
    assert_eq!(parsed.interface, "shard-store");
}

#[test]
fn byte_order_mark_is_not_stripped() {
    let err = state_err(
        "\u{feff}interface shard-store\nauthor engineer-a\n",
        "run.state",
    );
    let message = err.to_string();
    assert!(
        message.contains("line 1 does not name an interface"),
        "{message}"
    );
}
